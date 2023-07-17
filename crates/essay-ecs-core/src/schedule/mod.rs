mod executor;
mod system;
mod planner;
mod unsafe_cell;
mod multithreaded;
mod plan;
mod thread_pool;
mod phase;
mod preorder;
mod schedule;

use essay_ecs_core_macros::Phase;

pub use planner::SystemMeta;

pub use schedule::{
    Schedules, Schedule, ScheduleLabel, BoxedLabel,
};

pub use system::{
    IntoSystemConfig,
};

pub use executor::{
    Executors, Executor, ExecutorFactory,
};

pub use unsafe_cell::UnsafeWorld;

pub use phase::{
    DefaultPhase, Phase, IntoPhaseConfig, IntoPhaseConfigs,
};
