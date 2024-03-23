use std::sync::Arc;

use fixedbitset::FixedBitSet;

use crate::{
    Schedule, Store,
    system::SystemId,
};    

use super::{
    thread_pool::{ThreadPool, TaskSender, ThreadPoolBuilder}, 
    plan::Plan, 
    schedule::{ScheduleErr},
    executor::{Executor, ExecutorFactory},
    unsafe_cell::UnsafeSendCell, UnsafeStore
};

type ArcWorld = Arc<UnsafeSendCell<Option<UnsafeStore>>>;
type ArcSchedule = Arc<UnsafeSendCell<Option<Schedule>>>;

pub struct MultithreadedExecutorFactory;

impl ExecutorFactory for MultithreadedExecutorFactory {
    fn create(&self, plan: Plan) -> Box<dyn Executor> {
        Box::new(MultithreadedExecutor::new(plan))
    }

    fn box_clone(&self) -> Box<dyn ExecutorFactory> {
        Box::new(MultithreadedExecutorFactory {})
    }
}

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
    pub fn new(plan: Plan) -> Self {
        let arc_schedule: ArcSchedule = Arc::new(UnsafeSendCell::new(None));
        let arc_world: ArcWorld = Arc::new(UnsafeSendCell::new(None));

        let parent_task = ParentTask {
            plan,
            schedule: arc_schedule.clone(),
            world: arc_world.clone(),
        };

        let arc_schedule_child: ArcSchedule = Arc::clone(&arc_schedule);
        let arc_world_child: ArcWorld = Arc::clone(&arc_world);

        let pool = ThreadPoolBuilder::new().parent(
            move |sender| {
                parent_task.run(&sender).unwrap()
        }).child(move || {
            let child_task = ChildTask::new(
                Arc::clone(&arc_schedule_child),
                Arc::clone(&arc_world_child),
            );

            Box::new(move |s| { child_task.run(s).unwrap(); })
        }).build();

        Self {
            thread_pool: Some(pool),
            schedule: arc_schedule,
            world: arc_world,
        }
    }

    fn close(&mut self) {
        if let Some(mut pool) = self.thread_pool.take() {
            pool.close().unwrap();
        }
    }
}

impl Executor for MultithreadedExecutor {
    fn run(
        &mut self, 
        schedule: Schedule, 
        world: Store
    ) -> Result<(Schedule, Store), super::schedule::ScheduleErr> {
        match &self.thread_pool {
            Some(thread_pool) => { 
                unsafe {
                    self.world.as_mut().replace(UnsafeStore::new(world));
                    self.schedule.as_mut().replace(schedule);
                }

                thread_pool.start()?;
    
                let world = unsafe { self.world.as_mut().take() };
                let schedule = unsafe { self.schedule.as_mut().take() };
    
                Ok((schedule.unwrap(), world.unwrap().take()))
            },
            None => { panic!("thread pool is closed"); }
        }
    }
}

impl Drop for MultithreadedExecutor {
    fn drop(&mut self) {
        self.close();
    }
}

impl ParentTask {
    fn run(&self, sender: &TaskSender) -> Result<(),ScheduleErr> {
        if let Some(schedule) = unsafe { self.schedule.as_mut() } {
            if let Some(world) = unsafe { self.world.as_mut() } {
                return self.run_impl(sender, schedule, world)
            }
        }

        panic!("unset world");
    }

    fn run_impl(
        &self, 
        sender: &TaskSender,
        schedule: &mut Schedule,
        world: &mut UnsafeStore
    ) -> Result<(), ScheduleErr> {
        let n = self.plan.len();
        let mut n_active: usize = 0;
        let mut n_remaining = self.plan.len();
        let mut n_incoming = self.plan.n_incoming().clone();
        let mut n_ready: usize = 0;
        let mut n_child: usize = 0;

        let mut ready = FixedBitSet::with_capacity(n);

        for (i, n) in n_incoming.iter().enumerate() {
            if *n == 0 {
                ready.set(i, true);
                n_ready += 1;
            }
        }

        let mut started = Vec::<usize>::new();
        let mut completed = Vec::<SystemId>::new();

        while n_remaining + n_active > 0 {
            assert!(n_ready + n_active > 0);

            for order_id in ready.ones() {
                let id = self.plan.system_id(order_id);

                started.push(order_id);
                n_active += 1;

                let meta = schedule.meta(id);

                if meta.is_marker() {
                    completed.push(id);
                } else if meta.is_exclusive() {
                    assert_eq!(n_active, 1);

                    unsafe { schedule.run_system(id, world); }

                    completed.push(id);
                } else if n_ready == 1 && n_active == 1 {
                    unsafe { schedule.run_system(id, world); }

                    completed.push(id);
                } else {
                    sender.send(id);

                    n_child += 1;
                }
            }

            for order_id in started.drain(..) {
                ready.set(order_id, false);
                n_ready -= 1;
            }

            if n_child > 0 {
                sender.flush();

                n_child -= self.read_completed(sender, &mut completed);
            }

            for id in completed.drain(..) {
                n_ready += self.update_ready(id, &mut n_incoming, &mut ready);

                n_active -= 1;
                n_remaining -= 1;
            }

        }

        Ok(())
    }

    fn read_completed(
        &self, 
        sender: &TaskSender, 
        completed: &mut Vec<SystemId>
    ) -> usize {
        // TODO! multiread when available
        let id = sender.read(); 

        completed.push(id);
        1
    }

    fn update_ready(
        &self, 
        id: SystemId, 
        n_incoming: &mut Vec<usize>,
        ready: &mut FixedBitSet
    ) -> usize {
        let mut n_ready = 0;

        for outgoing in self.plan.outgoing(id) {
            n_incoming[*outgoing] -= 1;

            if n_incoming[*outgoing] == 0 {
                ready.set(*outgoing, true);
                n_ready += 1;
            }
        }

        n_ready
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

    fn run(&self, id: SystemId) -> Result<(), ScheduleErr> {
        if let Some(schedule) = unsafe { self.schedule.get_ref() } {
            if let Some(world) = unsafe { self.world.get_ref() } {
                unsafe { schedule.run_unsafe(id, world); }

                return Ok(());
            }
        }

        panic!("unset world");
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration, sync::{Arc, Mutex}};

    use crate::{Store, Schedule, 
        schedule::{Phase,IntoPhaseConfigs, 
            Executor, ExecutorFactory}, IntoSystemConfig,
    };

    use super::{MultithreadedExecutor, MultithreadedExecutorFactory};

    #[test]
    fn two_concurrent_no_phase() {
        let mut schedule = Schedule::new();
        let mut world = Store::new();

        let value = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[S"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S]"));
        });

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[S"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S]"));
        });

        schedule.init(&mut world);

        let factory = MultithreadedExecutorFactory;
        let mut exec = factory.create(schedule.plan());        

        (schedule, world) = exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[S, [S, S], S]");

        exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[S, [S, S], S]");
    }

    #[test]
    fn two_concurrent_phase_b() {
        let mut schedule = Schedule::new();
        schedule.add_phases((
            TestPhase::A,
            TestPhase::B,
            TestPhase::C,
        ).chain());
        //schedule.set_default_phase(TestPhase::B);

        let mut world = Store::new();

        let value = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        schedule.init(&mut world);

        let mut exec = MultithreadedExecutor::new(schedule.plan());        

        (schedule, world) = exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[B, [B, B], B]");

        exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[B, [B, B], B]");
    }

    #[test]
    fn two_sequential_phase_a_b() {
        let mut schedule = Schedule::new();
        schedule.add_phases((
            TestPhase::A,
            TestPhase::B,
            TestPhase::C,
        ).chain());
        //schedule.set_default_phase(TestPhase::B);

        let mut world = Store::new();

        let value = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = value.clone();
        schedule.add_system((move || {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        }).phase(TestPhase::A));

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        schedule.init(&mut world);

        let mut exec = MultithreadedExecutor::new(schedule.plan());        

        (schedule, world) = exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[A, A], [B, B]");

        exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[A, A], [B, B]");
    }

    #[test]
    fn two_sequential_phase_b_c() {
        let mut schedule = Schedule::new();
        schedule.add_phases((
            TestPhase::A,
            TestPhase::B,
            TestPhase::C,
        ).chain());
        //schedule.set_default_phase(TestPhase::B);

        let mut world = Store::new();

        let value = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = value.clone();
        schedule.add_system((move || {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        }).phase(TestPhase::C));

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        schedule.init(&mut world);

        let mut exec = MultithreadedExecutor::new(schedule.plan());        

        (schedule, world) = exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[B, B], [C, C]");

        exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[B, B], [C, C]");
    }


    #[test]
    #[should_panic(expected="parent panic received by thread pool")]
    fn system_panic() {
        let mut schedule = Schedule::new();
        let mut world = Store::new();

        let value = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[P"));
            panic!("system panic");
        });

        let ptr = value.clone();
        schedule.add_system(move || {
            push(&ptr, format!("[S"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S]"));
        });

        schedule.init(&mut world);

        let factory = MultithreadedExecutorFactory;
        let mut exec = factory.create(schedule.plan());        

        (schedule, world) = exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[S, [S, S], S]");

        exec.run(schedule, world).unwrap();

        assert_eq!(take(&value), "[S, [S, S], S]");
    }

    fn push(arc: &Arc<Mutex<Vec<String>>>, value: String) {
        arc.lock().unwrap().push(value);
    }

    fn take(arc: &Arc<Mutex<Vec<String>>>) -> String {
        let values: Vec<String> = arc.lock().unwrap().drain(..).collect();

        values.join(", ")
    }

    #[derive(Debug, Clone, PartialEq, Hash, Eq)]
    enum TestPhase {
        A,
        B,
        C,
    }

    impl Phase for TestPhase {
        fn box_clone(&self) -> Box<dyn Phase> {
            Box::new(Clone::clone(self))
        }
    }
}