use std::{sync::{Arc, Mutex}, cell::UnsafeCell};

use fixedbitset::FixedBitSet;

use crate::{Schedule, World, schedule::{schedule::SystemId, future}};

use super::{thread_pool::{ThreadPool, TaskSender, ThreadPoolBuilder}, plan::{SystemPlan, Plan}, schedule::SystemItem, System, cell::SyncUnsafeCell};

pub struct MultithreadedExecutor {
    thread_pool: Option<ThreadPool>,
    
    env: Arc<Mutex<EnvHolder>>,
}

struct ParentTask {
    plan: Plan,

    env: Arc<Mutex<EnvHolder>>,
}

struct EnvHolder {
    world: Option<UnsafeWorld>,
    schedule: Option<UnsafeSchedule>,
}


impl MultithreadedExecutor {
    pub fn new(schedule: &Schedule) -> Self {
        let env_holder = Arc::new(Mutex::new(EnvHolder::new()));

        let parent_task = ParentTask {
            plan: schedule.plan(),
            env: Arc::clone(&env_holder),
        };

        let pool = ThreadPoolBuilder::new().parent(
            move |sender| {
                parent_task.run(&sender);
        }).build();

        Self {
            thread_pool: Some(pool),
            env: env_holder,
        }
    }

    fn run(&self, world: World, schedule: Schedule) -> (World, Schedule) {
        match &self.thread_pool {
            Some(thread_pool) => { 
                self.env.lock().unwrap().world = Some(UnsafeWorld(world));
                self.env.lock().unwrap().schedule = Some(UnsafeSchedule(schedule));

                thread_pool.start(); 

                let world = self.env.lock().unwrap().world.take();
                let schedule = self.env.lock().unwrap().schedule.take();

                (world.unwrap().take(), schedule.unwrap().take())
            },
            None => { panic!("thread pool is closed"); }
        }
    }

    fn close(&mut self) {
        if let Some(mut pool) = self.thread_pool.take() {
            pool.close();
        }
    }
}

impl Drop for MultithreadedExecutor {
    fn drop(&mut self) {
        self.close();
    }
}

struct ParentGuard<'a> {
    parent: &'a ParentTask,
    world: Option<UnsafeWorld>,
    schedule: Option<Schedule>,
}

impl<'a> ParentGuard<'a> {
    fn new(
        parent: &'a ParentTask,
        world: UnsafeWorld,
        schedule: Schedule
    ) -> Self {
        Self {
            parent,
            world: Some(world),
            schedule: Some(schedule)
        }
    }
}

impl Drop for ParentGuard<'_> {
    fn drop(&mut self) {
        if let Some(world) = self.world.take() {
            self.parent.env.lock().unwrap().world = Some(world);
        }

        if let Some(schedule) = self.schedule.take() {
            self.parent.env.lock().unwrap().schedule = Some(UnsafeSchedule(schedule));
        }
    }
}


impl ParentTask {
    fn run(&self, sender: &TaskSender) {
        let world = self.env.lock().unwrap().world.take().unwrap();
        let schedule = self.env.lock().unwrap().schedule.take().unwrap().take();

        let mut guard = ParentGuard::new(&self, world, schedule);

        if let Some(world) = &mut guard.world {
            if let Some(schedule) = &mut guard.schedule {
                self.run_impl(sender, schedule, world);
            }
        }
    }

    fn run_impl<'a>(
        &self, 
        sender: &'a TaskSender<'a>, 
        schedule: &'a mut Schedule,
        world: &'a mut UnsafeWorld
    ) {
        let n = self.plan.len();
        let mut n_active: usize = 0;
        let mut n_remaining = self.plan.len();
        let mut n_incoming = self.plan.n_incoming().clone();
        let mut n_ready: usize = 0;

        let mut ready = FixedBitSet::with_capacity(n);

        for (i, n) in n_incoming.iter().enumerate() {
            if *n == 0 {
                ready.set(i, true);
                n_ready += 1;
            }
        }

        let mut started = FixedBitSet::with_capacity(n);

        while n_remaining > 0 {
            started.clear();

            assert!(n_ready > 0);

            while n_ready > 0 {
                for i in ready.ones() {
                    started.set(i, true);
                    n_ready -= 1;

                    let system_id = SystemId(i);

                    let system_item = schedule.system(system_id);

                    let system = system_item.system();

                    if ! system_item.meta().is_exclusive() {
                        /*
                        let task = async move {
                            unsafe { system.get_mut().run_unsafe(world.get_mut()); }
                        };
                        */
                        self.spawn_task(sender, system, system_id, world);

                    } else if system_item.meta().is_flush() {
                        schedule.flush(world.get_mut())
                    } else {
                        system.as_mut().run(world.get_mut());
                    }

                    n_remaining -= 1;
                }
            }

            ready.difference_with(&started);
        }
        println!("start: {:?} ready:{:?}", n_remaining, n_ready);
    }

    fn spawn_task<'a>(
        &self,
        sender: &TaskSender<'a>, 
        system: &'a SyncUnsafeCell<Box<dyn System<Out=()>>>,
        system_id: SystemId,
        world: &'a UnsafeWorld
    ) {
        sender.send(system_id);
    }
}

impl EnvHolder {
    pub fn new() -> Self {
        Self {
            world: None,
            schedule: None,
        }
    }
}

struct UnsafeWorld(World);

impl UnsafeWorld {
    fn new(world: World) -> Self {
        UnsafeWorld(world)
    }

    fn get_mut(&mut self) -> &mut World {
        &mut self.0
    }

    fn take(self) -> World {
        self.0
    }
}

unsafe impl Send for UnsafeWorld {}
unsafe impl Sync for UnsafeWorld {}

unsafe impl Send for SystemItem {}
unsafe impl Sync for SystemItem {}

struct UnsafeSchedule(Schedule);

impl UnsafeSchedule {
    fn new(schedule: Schedule) -> Self {
        UnsafeSchedule(schedule)
    }

    fn take(self) -> Schedule {
        self.0
    }
}

unsafe impl Send for UnsafeSchedule {}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::{World, Schedule};

    use super::MultithreadedExecutor;

    #[test]
    fn test() {
        let mut schedule = Schedule::new();
        let mut world = World::new();

        schedule.add_system(move || println!("system-1"));
        schedule.add_system(move || println!("system-2"));

        schedule.init(&mut world);

        let exec = MultithreadedExecutor::new(&schedule);        

        (world, schedule) = exec.run(world, schedule);

        println!("world, schedule");

        thread::sleep(Duration::from_millis(100));
        println!("complete");
    }
}