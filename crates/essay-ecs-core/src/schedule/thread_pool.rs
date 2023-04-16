use core::fmt;
use std::{
    thread::{Thread, self, JoinHandle}, 
    sync::{mpsc::{self, Receiver, Sender}, Arc}, 
    time::Duration
};

use concurrent_queue::{ConcurrentQueue, PopError};

use super::schedule::SystemId;

pub struct ThreadPoolBuilder {
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
    Start(MainClosure),
    Complete,
    Exit,
}

enum TaskMessage {
    Start(TaskClosure),
    Exit,
}

pub struct ParentThread {
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
            n_threads: None,
        }
    }

    pub fn n_threads(mut self, n_threads: usize) -> Self {
        assert!(n_threads > 0);

        self.n_threads = Some(n_threads);

        self
    }

    pub fn build(self) -> ThreadPool {
        let parallelism = thread::available_parallelism().unwrap();
        println!("parallel {:?}", parallelism);

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

        for i in 0..n_threads {
            let mut task_thread = ChildThread::new(
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
    pub fn start(&mut self, fun: impl FnOnce(&TaskSender) + Send + 'static) {
        self.executive_sender.send(MainMessage::Start(Box::new(fun))).unwrap();
        
        loop {
            match self.executive_reader.recv() {
                Ok(MainMessage::Exit) => {
                    self.close();
                    panic!("unexpected exit");
                }
                Ok(MainMessage::Complete) => {
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
                Ok(MainMessage::Start(task)) => {
                    task(&sender);

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
        registry: Arc<Registry>, 
        sender: Sender<SystemId>,
        index: usize
    ) -> Self {
        Self {
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
                TaskMessage::Start(fun) => {
                    let id = fun();
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
    pub fn send(&self, fun: impl FnOnce() -> SystemId + Send + 'static) {
        self.thread.registry.queue.push(TaskMessage::Start(Box::new(fun))).unwrap();
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
        let mut pool = ThreadPoolBuilder::new().n_threads(2).build();

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        pool.start(move |sender| {
            ptr.lock().unwrap().push(format!("[P"));

            let ptr2 = ptr.clone();
            sender.send(move || { 
                ptr2.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr2.lock().unwrap().push(format!("C]"));
                SystemId(0) 
            });

            let ptr2 = ptr.clone();
            sender.send(move || { 
                ptr2.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr2.lock().unwrap().push(format!("C]"));
                SystemId(1) 
            });
            sender.flush();

            sender.read();
            sender.read();

            ptr.lock().unwrap().push(format!("P]"));
        });

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, [C, C], C], P]");

        pool.close();
    }

    #[test]
    fn two_tasks_one_thread() {
        let mut pool = ThreadPoolBuilder::new().n_threads(1).build();

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        pool.start(move |sender| {
            ptr.lock().unwrap().push(format!("[P"));

            let ptr2 = ptr.clone();
            sender.send(move || { 
                ptr2.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr2.lock().unwrap().push(format!("C]"));
                SystemId(0) 
            });

            let ptr2 = ptr.clone();
            sender.send(move || { 
                ptr2.lock().unwrap().push(format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr2.lock().unwrap().push(format!("C]"));
                SystemId(1) 
            });
            sender.flush();

            sender.read();
            sender.read();

            ptr.lock().unwrap().push(format!("P]"));
        });

        let list: Vec<String> = values.lock().unwrap().drain(..).collect();
        assert_eq!(list.join(", "), "[P, [C, C], [C, C], P]");

        pool.close();
    }
}