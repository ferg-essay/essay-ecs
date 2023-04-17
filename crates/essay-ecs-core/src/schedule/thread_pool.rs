use core::fmt;
use std::{
    thread::{Thread, self, JoinHandle}, 
    sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, 
    time::Duration, future::Future
};

use concurrent_queue::{ConcurrentQueue, PopError};

use crate::{World, Schedule};

use super::schedule::SystemId;

pub struct ThreadPoolBuilder {
    parent_task: Option<Box<dyn Fn(&TaskSender) + Send>>,
    child_task_builder: Option<Box<dyn Fn() -> Box<dyn Fn(SystemId) + Send>>>,
    n_threads: Option<usize>,
}

pub struct ThreadPool {
    threads: Vec<Thread>,
    executive: Option<JoinHandle<()>>,

    executive_sender: Sender<MainMessage>,
    executive_reader: Receiver<MainMessage>,
}

pub struct TaskSender<'a> {
    thread: &'a ParentThread,
}

type MainClosure = Box<dyn FnOnce(&TaskSender) + Send>;
type TaskClosure = Box<dyn FnOnce() -> SystemId + Send>;

enum MainMessage {
    Start,
    Complete,
    Exit,
}

enum TaskMessage {
    Start(SystemId),
    Exit,
}

pub struct ParentThread {
    task: Box<dyn Fn(&TaskSender) + Send>,

    main_reader: Receiver<MainMessage>,
    main_sender: Sender<MainMessage>,

    registry: Arc<Registry>,

    task_receiver: Receiver<SystemId>,
    handles: Vec<JoinHandle<()>>,
}

struct Registry {
    queue: ConcurrentQueue<TaskMessage>,
    tasks: Vec<TaskInfo>,
}

struct TaskInfo {
    handle: Option<JoinHandle<()>>,
}

impl TaskInfo {
    pub fn new() -> Self {
        TaskInfo {
            handle: None,
        }
    }
}

struct ChildThread {
    task: Box<dyn Fn(SystemId) + Send>,
    registry: Arc<Registry>,
    sender: Sender<SystemId>,
    index: usize,
}

//
// Implementation
//

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
        F: Fn(&TaskSender) + Send + 'static
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

    pub fn n_threads(mut self, n_threads: usize) -> Self {
        assert!(n_threads > 0);

        self.n_threads = Some(n_threads);

        self
    }

    pub fn build(self) -> ThreadPool {
        assert!(! self.parent_task.is_none());
        assert!(! self.child_task_builder.is_none());

        let parallelism = thread::available_parallelism().unwrap();
        

        let (executive_sender, main_reader) = mpsc::channel();
        let (main_sender, executive_reader) = mpsc::channel();

        let (task_sender, task_reader) = mpsc::channel();

        let n_threads = match self.n_threads {
            Some(n_threads) => n_threads,
            None => 2,
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

        for i in 0..n_threads {
            let mut task_thread = ChildThread::new(
                builder(),
                Arc::clone(&registry), 
                task_sender.clone(),
                i
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
            executive.run();
        });

        ThreadPool {
            threads: Vec::new(),

            executive: Some(handle),

            executive_sender,
            executive_reader,
        }
    }
}

impl ThreadPool {
    pub fn start(&self) {
        self.executive_sender.send(MainMessage::Start).unwrap();
        
        loop {
            match self.executive_reader.recv() {
                Ok(MainMessage::Exit) => {
                    panic!("unexpected exit");
                }
                Ok(MainMessage::Complete) => {
                    println!("main complete");
                    
                    return;
                }
                Ok(_) => {
                    panic!("invalid executive message");
                }
                Err(err) => {
                    panic!("executor receive error {:?}", err);
                }
            }
        }
    }

    pub fn close(&mut self) {
        match self.executive.take() {
            Some(handle) => {
                self.executive_sender.send(MainMessage::Exit).unwrap();
                // TODO: timed?
                handle.join().unwrap();
            },
            None => {},
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.close();
    }
}

impl ParentThread {
    pub fn run(&mut self) {
        let sender = TaskSender { thread: &self };
        loop {
            match self.main_reader.recv() {
                Ok(MainMessage::Start) => {
                    (self.task)(&sender);
                    println!("sender complete");

                    self.main_sender.send(MainMessage::Complete).unwrap();
                }
                Ok(MainMessage::Exit) => {
                    self.main_sender.send(MainMessage::Exit).unwrap();
                    return;
                }
                Ok(_) => {
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

impl ChildThread {
    pub fn new(
        task: Box<dyn Fn(SystemId) + Send>,
        registry: Arc<Registry>, 
        sender: Sender<SystemId>,
        index: usize
    ) -> Self {
        Self {
            task,
            registry,
            sender,
            index,
        }
    }

    pub fn run(&mut self) {
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

                    self.sender.send(id).unwrap();
                },
                TaskMessage::Exit => {
                    return;
                }
            }
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
        self.thread.task_receiver.recv().unwrap()
    }

    pub fn try_read(&self) -> Option<SystemId> {
        match self.thread.task_receiver.try_recv() {
            Ok(id) => Some(id),
            Err(msg) => { println!("msg {:?}", msg); None }
        }
    }
}

impl fmt::Debug for TaskMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start(_arg0) => f.debug_tuple("Start").finish(),
            Self::Exit => write!(f, "Exit"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration, sync::{Arc, Mutex}};

    use crate::schedule::schedule::SystemId;

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
        }).child(move || {
            let ptr3 = ptr2.clone();

            Box::new(move |s| { 
                ptr3.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr3.lock().unwrap().push(format!("C]"));
            })
        }).n_threads(2)
        .build();

        pool.start();

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, [C, C], C], P]");

        pool.close();
    }

    #[test]
    fn two_tasks_one_thread() {
        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        let ptr2 = values.clone();

        let mut pool = ThreadPoolBuilder::new().parent(
        move |sender| {
            ptr.lock().unwrap().push(format!("[P"));

            let ptr2 = ptr.clone();
            sender.send(SystemId(0));
            sender.send(SystemId(1));
            sender.flush();

            sender.read();
            sender.read();

            ptr.lock().unwrap().push(format!("P]"));
        }).child(move || {
            let ptr3 = ptr2.clone();

            Box::new(move |s| { 
                ptr3.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr3.lock().unwrap().push(format!("C]"));
            })
        }).n_threads(1).build();

        pool.start();

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, C], [C, C], P]");

        pool.close();
    }
}