use core::{fmt, panic};
use std::{
    thread::{self, JoinHandle}, 
    sync::{mpsc::{self, Receiver, Sender}, Arc}, 
};

use concurrent_queue::{ConcurrentQueue, PopError};
use log::info;

use crate::{
    error::{Error, Result},
    system::SystemId
};

//
// ThreadPoolBuilder
//

pub struct ThreadPoolBuilder {
    parent_task: Option<Box<dyn Fn(&TaskSender) -> Result<()> + Send>>,
    child_task_builder: Option<Box<dyn Fn() -> Box<dyn Fn(SystemId) + Send>>>,
    n_threads: Option<usize>,
}

impl ThreadPoolBuilder {
    pub fn new() -> Self {
        Self {
            parent_task: None,
            child_task_builder: None,
            n_threads: None,
        }
    }

    pub fn parent<F>(mut self, task: F) -> Self
    where
        F: Fn(&TaskSender) -> Result<()> + Send + 'static
    {
        self.parent_task = Some(Box::new(task));

        self
    }

    pub fn child<F>(mut self, task: F) -> Self
    where
        F: Fn()->Box<dyn Fn(SystemId) + Send> + 'static
    {
        self.child_task_builder = Some(Box::new(task));

        self
    }

    pub fn _n_threads(mut self, n_threads: usize) -> Self {
        assert!(n_threads > 0);

        self.n_threads = Some(n_threads);

        self
    }

    pub fn build(self) -> ThreadPool {
        assert!(! self.parent_task.is_none());
        assert!(! self.child_task_builder.is_none());

        let (executive_sender, main_reader) = mpsc::channel();
        let (main_sender, executive_reader) = mpsc::channel();

        let (task_sender, task_reader) = mpsc::channel();

        let n_threads = match self.n_threads {
            Some(n_threads) => n_threads,
            None => usize::from(thread::available_parallelism().unwrap()),
        };

        let mut registry = Registry {
            queue: ConcurrentQueue::unbounded(),
            tasks: Vec::new(),
        };

        for _ in 0..n_threads {
            registry.tasks.push(TaskInfo::new());
        }

        let registry = Arc::new(registry);

        let mut handles = Vec::<JoinHandle<()>>::new();

        let builder = self.child_task_builder.unwrap();

        for _ in 0..n_threads {
            let mut task_thread = ChildThread::new(
                builder(),
                Arc::clone(&registry), 
                task_sender.clone(),
            );

            let handle = thread::spawn(move || {
                task_thread.run();
            });

            handles.push(handle);
        }

        let mut executive = ParentThread {
            task: self.parent_task.unwrap(),

            main_reader,
            main_sender,

            registry,

            task_receiver: task_reader,
            handles,
        };

        let handle = thread::spawn(move || {
            executive.run().unwrap();
        });

        ThreadPool {
            //threads: Vec::new(),

            executive: Some(handle),

            executive_sender,
            executive_reader,
        }
    }
}

pub struct ThreadPool {
    //threads: Vec<Thread>,
    executive: Option<JoinHandle<()>>,

    executive_sender: Sender<MainMessage>,
    executive_reader: Receiver<MainMessage>,
}

struct ChildThread {
    task: Box<dyn Fn(SystemId) + Send>,
    registry: Arc<Registry>,
    sender: Sender<Result<SystemId>>,
}

pub struct TaskSender<'a> {
    thread: &'a ParentThread,
}

#[derive(Debug)]
enum MainMessage {
    Start,
    Complete,
    Exit,
    Error(Error),
    Panic(Error),
}

enum TaskMessage {
    Start(SystemId),
    _Exit,
}

struct Registry {
    queue: ConcurrentQueue<TaskMessage>,
    tasks: Vec<TaskInfo>,
}

impl Registry {
    fn close(&self) {
        self.queue.close();
    }
}

struct TaskInfo {
    _handle: Option<JoinHandle<()>>,
}

impl TaskInfo {
    pub fn new() -> Self {
        TaskInfo {
            _handle: None,
        }
    }
}

//
// Implementation
//

impl ThreadPool {
    pub fn start(&self) -> Result<()> {
        if let Err(err) = self.executive_sender.send(MainMessage::Start) {
            return Err(err.to_string().into());
        }
        
        loop {
            match self.executive_reader.recv() {
                Ok(MainMessage::Exit) => {
                    return Err(format!("unexpected exit\n\tin {}:{}", file!(), line!()).into());
                    // panic!("unexpected exit");
                }
                Ok(MainMessage::Complete) => {
                    return Ok(());
                }
                Ok(MainMessage::Panic(error)) => {
                    return Err(error);
                }
                Ok(MainMessage::Error(error)) => {
                    return Err(error);
                }
                Ok(msg) => {
                    return Err(format!("invalid executive message {:?}", msg).into());
                }
                Err(err) => {
                    return Err(format!("{:?}\n\tat {}:{}", err, file!(), line!()).into())
                }
            }
        }
    }

    pub fn close(&mut self) -> Result<()> {
        match self.executive.take() {
            Some(handle) => {
                match self.executive_sender.send(MainMessage::Exit) {
                    Ok(_) => {},
                    Err(err) => {
                        info!("error sending exit {:#?}", err);
                    },
                };

                // TODO: timed?
                match handle.join() {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        Err(format!("{:?}\n\tin ThreadPool::close()", err).into())
                    },
                }
            },
            None => Ok(()),
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        match self.close() {
            Ok(_) => {},
            Err(err) => { info!("error while closing {:#?}", err) }
        };
    }
}

//
// ParentThread
//

pub struct ParentThread {
    task: Box<dyn Fn(&TaskSender) -> Result<()> + Send>,

    main_reader: Receiver<MainMessage>,
    main_sender: Sender<MainMessage>,

    registry: Arc<Registry>,

    task_receiver: Receiver<Result<SystemId>>,
    handles: Vec<JoinHandle<()>>,
}

impl ParentThread {
    pub fn run(&mut self) -> Result<()> {
        let mut guard = ParentGuard::new(self);

        let sender = TaskSender { thread: &self };

        loop {
            match self.main_reader.recv() {
                Ok(MainMessage::Start) => {
                    match (self.task)(&sender) {
                        Ok(_) => {
                            match self.main_sender.send(MainMessage::Complete) {
                                Ok(_) => {},
                                Err(err) => {
                                    return Err(format!("error sending MainMessage::Complete {:#?}", err).into());
                                }
                            }
                        }
                        Err(error) => {
                            match self.main_sender.send(MainMessage::Error(error)) {
                                Ok(_) => {},
                                Err(err) => {
                                    return Err(format!("error sending MainMessage::Error {:#?}", err).into());
                               }
                            }
                        }
                    }
                }
                Ok(MainMessage::Exit) => {
                    sender.close();
                    match self.main_sender.send(MainMessage::Complete) {
                        Ok(_) => {},
                        Err(err) => {
                            return Err(format!("error sending MainMessage::Exit {:#?}", err).into());
                        }
                    }
                    guard.close();
                    return Ok(());
                }
                Ok(_) => {
                    self.main_sender.send(MainMessage::Panic("invalid executor".into())).unwrap();
                    panic!("invalid executor message");
                }
                Err(err) => {
                    panic!("executor receive error {:?}", err);
                }
            }
        }
    }

    fn unpark(&self) {
        for h in &self.handles {
            h.thread().unpark();
        }
    }
}

struct ParentGuard<'a> {
    parent: &'a ParentThread,
    is_close: bool,
}

impl<'a> ParentGuard<'a> {
    fn new(parent: &'a ParentThread) -> Self {
        Self {
            parent,
            is_close: false,
        }
    }

    fn close(&mut self) {
        self.is_close = true;
    }
}

impl Drop for ParentGuard<'_> {
    fn drop(&mut self) {
        if ! self.is_close {
            self.parent.main_sender.send(MainMessage::Panic(format!("parent not closed\n\tat {}:{}", file!(), line!()).into())).unwrap();
        }
    }
}

impl ChildThread {
    pub fn new(
        task: Box<dyn Fn(SystemId) + Send>,
        registry: Arc<Registry>, 
        sender: Sender<Result<SystemId>>,
    ) -> Self {
        Self {
            task,
            registry,
            sender,
        }
    }

    pub fn run(&mut self) {
        let mut guard = ChildGuard::new(self);

        let queue = &self.registry.queue;

        loop {
            let msg = match queue.pop() {
                Ok(msg) => msg,
                Err(PopError::Empty) => {
                    thread::park();
                    continue;
                }
                Err(err) => panic!("unknown queue error {:?}", err)
            };

            match msg {
                TaskMessage::Start(id) => {
                    (self.task)(id);

                    self.sender.send(Ok(id)).unwrap();
                },
                TaskMessage::_Exit => {
                    guard.close();
                    return;
                }
            }
        }
    }
}

struct ChildGuard<'a> {
    child: &'a ChildThread,
    is_close: bool,
}

impl<'a> ChildGuard<'a> {
    fn new(child: &'a ChildThread) -> Self {
        Self {
            child,
            is_close: false,
        }
    }

    fn close(&mut self) {
        self.is_close = true;
    }
}

impl Drop for ChildGuard<'_> {
    fn drop(&mut self) {
        if ! self.is_close {
            self.child.sender.send(Err("ChildPanic".into())).unwrap();
            self.child.registry.close();
        }
    }
}

impl<'a> TaskSender<'a> {
    pub fn send(&self, system_id: SystemId)
    {
        self.thread.registry.queue.push(TaskMessage::Start(system_id)).unwrap();
    }

    pub fn flush(&self) {
        self.thread.unpark();
    }

    pub fn read(&self) -> SystemId {
        self.thread.task_receiver.recv().unwrap().unwrap()
    }

    pub fn _try_read(&self) -> Option<SystemId> {
        match self.thread.task_receiver.try_recv() {
            Ok(id) => Some(id.unwrap()),
            Err(msg) => { panic!("msg {:?}", msg); }
        }
    }

    fn close(&self) {
        self.thread.registry.close();
    }
}

impl<'a> Drop for TaskSender<'a> {
    fn drop(&mut self) {
        self.thread.registry.close();
    }
}

impl fmt::Debug for TaskMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start(_arg0) => f.debug_tuple("Start").finish(),
            Self::_Exit => write!(f, "Exit"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration, sync::{Arc, Mutex}};

    use crate::system::SystemId;

    use super::ThreadPoolBuilder;

    #[test]
    fn two_tasks_two_threads() {
        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        let ptr2 = values.clone();
    
        let mut pool = ThreadPoolBuilder::new().parent(
            move |sender| {
            ptr.lock().unwrap().push(format!("[P"));

            sender.send(SystemId(0));
            sender.send(SystemId(1));
            sender.flush();

            sender.read();
            sender.read();

            ptr.lock().unwrap().push(format!("P]"));

            Ok(())
        }).child(move || {
            let ptr3 = ptr2.clone();

            Box::new(move |_s| { 
                ptr3.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr3.lock().unwrap().push(format!("C]"));
            })
        })._n_threads(2)
        .build();

        pool.start().unwrap();

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, [C, C], C], P]");

        pool.close().unwrap();
    }

    #[test]
    fn two_tasks_one_thread() {
        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        let ptr2 = values.clone();

        let mut pool = ThreadPoolBuilder::new().parent(
        move |sender| {
            ptr.lock().unwrap().push(format!("[P"));

            sender.send(SystemId(0));
            sender.send(SystemId(1));
            sender.flush();

            sender.read();
            sender.read();

            ptr.lock().unwrap().push(format!("P]"));

            Ok(())
        }).child(move || {
            let ptr3 = ptr2.clone();

            Box::new(move |_| { 
                ptr3.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr3.lock().unwrap().push(format!("C]"));
            })
        })._n_threads(1).build();

        pool.start().unwrap();

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, C], [C, C], P]");

        pool.close().unwrap();
    }

    #[test]
    #[should_panic]
    fn panic_in_parent() {
        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        let ptr2 = values.clone();

        let mut pool = ThreadPoolBuilder::new().parent(
        move |_sender| {
            ptr.lock().unwrap().push(format!("[P"));

            panic!("test parent panic");
        }).child(move || {
            let ptr3 = ptr2.clone();

            Box::new(move |_s| { 
                ptr3.lock().unwrap().push(format!("[C"));
                ptr3.lock().unwrap().push(format!("C]"));
            })
        }).build();

        pool.start().unwrap();

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, C], [C, C], P]");

        pool.close().unwrap();
    }

    #[test]
    #[should_panic]
    fn panic_in_child() {
        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        let ptr2 = values.clone();

        let mut pool = ThreadPoolBuilder::new().parent(
        move |sender| {
            ptr.lock().unwrap().push(format!("[P"));

            sender.send(SystemId(0));
            sender.send(SystemId(1));
            sender.flush();

            sender.read();
            sender.read();

            ptr.lock().unwrap().push(format!("P]"));

            Ok(())
        }).child(move || {
            let ptr3 = ptr2.clone();

            Box::new(move |_s| { 
                ptr3.lock().unwrap().push(format!("[C"));

                panic!("test child panic");
            })
        }).build();

        pool.start().unwrap();

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, C], [C, C], P]");

        pool.close().unwrap();
    }
}