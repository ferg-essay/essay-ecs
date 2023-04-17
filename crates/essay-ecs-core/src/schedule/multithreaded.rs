use std::{sync::{Arc, Mutex}, cell::UnsafeCell};

use fixedbitset::FixedBitSet;

use crate::{Schedule, World, schedule::{schedule::SystemId}};

use super::{thread_pool::{ThreadPool, TaskSender, ThreadPoolBuilder}, plan::{SystemPlan, Plan}, schedule::SystemItem, System, cell::SyncUnsafeCell};

type UnsafeWorld = UnsafeSend<World>;
type ArcWorld = Arc<UnsafeSend<Option<World>>>;

type UnsafeSchedule = UnsafeSend<Schedule>;
type ArcSchedule = Arc<UnsafeSend<Option<Schedule>>>;

pub struct MultithreadedExecutor {
    thread_pool: Option<ThreadPool>,
    
    schedule: ArcSchedule,
    world: ArcWorld,
}

struct ParentTask {
    plan: Plan,

    schedule: ArcSchedule,
    world: ArcWorld,
}

struct ChildTask {
    world: ArcWorld,
    schedule: ArcSchedule,
}

impl MultithreadedExecutor {
    pub fn new(schedule: &Schedule) -> Self {
        let arc_schedule: ArcSchedule = Arc::new(UnsafeSend::new(None));
        let arc_world: ArcWorld = Arc::new(UnsafeSend::new(None));

        let parent_task = ParentTask {
            plan: schedule.plan(),
            schedule: arc_schedule.clone(),
            world: arc_world.clone(),
        };

        let arc_schedule_child: ArcSchedule = Arc::clone(&arc_schedule);
        let arc_world_child: ArcWorld = Arc::clone(&arc_world);

        let pool = ThreadPoolBuilder::new().parent(
            move |sender| {
                parent_task.run(&sender);
        }).child(move || {
            let child_task = ChildTask::new(
                Arc::clone(&arc_schedule_child),
                Arc::clone(&arc_world_child),
            );

            Box::new(move |s| { child_task.run(s); })
        }).build();

        Self {
            thread_pool: Some(pool),
            schedule: arc_schedule,
            world: arc_world,
        }
    }

    fn run(&mut self, world: World, schedule: Schedule) -> (World, Schedule) {
        match &self.thread_pool {
            Some(thread_pool) => { 
                unsafe {
                    self.world.as_mut().replace(world);
                    self.schedule.as_mut().replace(schedule);
                }

                thread_pool.start(); 

                let world = unsafe { self.world.as_mut().take() };
                let schedule = unsafe { self.schedule.as_mut().take() };

                (world.unwrap(), schedule.unwrap())
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
/*
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
*/


impl ParentTask {
    fn run(&self, sender: &TaskSender) {
        if let Some(schedule) = unsafe { self.schedule.as_mut() } {
            if let Some(world) = unsafe { self.world.as_mut() } {
                self.run_impl(sender, schedule, world);

                return;
            }
        }

        panic!("unset world");
    }

    fn run_impl(
        &self, 
        sender: &TaskSender,
        schedule: &mut Schedule,
        world: &mut World
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

                    let system_item = unsafe { schedule.system(system_id) };

                    let system = system_item.system();

                    if ! system_item.meta().is_exclusive() {
                        sender.send(system_id);
                    } else if system_item.meta().is_flush() {
                        unsafe { schedule.flush(world) }
                    } else {
                        unsafe { system.as_mut().run(world); }
                    }

                    n_remaining -= 1;
                }
            }

            sender.flush();

            let system_id = sender.read();
            println!("system-id {:?}", system_id);

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
    }
}

impl ChildTask {
    fn new(
        schedule: ArcSchedule,
        world: ArcWorld,
    ) -> Self {
        Self {
            schedule,
            world
        }
    }

    fn run(&self, id: SystemId) {
        if let Some(schedule) = unsafe { self.schedule.as_mut() } {
            if let Some(world) = unsafe { self.world.as_mut() } {
                self.run_impl(id, schedule, world);

                return;
            }
        }

        panic!("unset world");
    }

    fn run_impl(
        &self, 
        id: SystemId, 
        schedule: &mut Schedule, 
        world: &mut World
    ) {
        println!("run {:?}", id);
    }
}

struct UnsafeSend<T>(UnsafeCell<T>);

impl<T> UnsafeSend<T> {
    fn new(value: T) -> Self {
        UnsafeSend(UnsafeCell::new(value))
    }

    pub(crate) fn as_ref(&self) -> &T {
        unsafe { self.0.get().as_ref().unwrap() }
    }

    pub(crate) unsafe fn as_mut(&self) -> &mut T {
        &mut *self.0.get()
    }

    fn take(self) -> T {
        self.0.into_inner()
    }
}

unsafe impl<T> Send for UnsafeSend<T> {}
unsafe impl<T> Sync for UnsafeSend<T> {}

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

        let mut exec = MultithreadedExecutor::new(&schedule);        

        (world, schedule) = exec.run(world, schedule);

        println!("world, schedule");

        thread::sleep(Duration::from_millis(100));
        println!("complete");
    }
}