use crate::{error::Result, Schedule, Store};

use super::{
    multithreaded::MultithreadedExecutor, 
    plan::Plan, 
    UnsafeStore, 
};


pub trait Executor: Send {
    fn run(
        &mut self, 
        schedule: Schedule, 
        store: Store
    ) -> Result<(Schedule, Store)>;
}

pub trait ExecutorFactory: Send + 'static {
    fn create(&self, plan: Plan) -> Box<dyn Executor>;

    fn box_clone(&self) -> Box<dyn ExecutorFactory>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Executors {
    Single,
    Multithreaded,
}

impl Default for Executors {
    fn default() -> Self {
        Executors::Multithreaded
    }
}

impl ExecutorFactory for Executors {
    fn create(&self, plan: Plan) -> Box<dyn Executor> {
        match self {
            Executors::Single => Box::new(SingleExecutor(plan)),
            Executors::Multithreaded => {
                Box::new(MultithreadedExecutor::new(plan))
            },
        }
    }

    fn box_clone(&self) -> Box<dyn ExecutorFactory> {
        Box::new(self.clone())
    }
}

impl Default for Box<dyn ExecutorFactory> {
    fn default() -> Self {
        Executors::default().box_clone()
    }
}
struct SingleExecutor(Plan);

impl Executor for SingleExecutor {
    fn run(
        &mut self, 
        mut schedule: Schedule, 
        world: Store
    ) -> Result<(Schedule, Store)> {
        let mut world = UnsafeStore::new(world);

        for id in self.0.order() {
            let meta = schedule.meta(*id);

            if meta.is_marker() {
                schedule.flush(&mut world);
            }
            else {
                unsafe { schedule.run_system(*id, &mut world)?; }
            }
        }

        Ok((schedule, world.take()))
    }
}
