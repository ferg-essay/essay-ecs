//use essay_ecs_macros::ScheduleLabel;

use essay_ecs_core::prelude::{ScheduleLabel, Phase};

mod plugin;
mod app;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreSchedule {
    Startup,
    Main,
    Outer,
}

#[derive(Phase, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreTaskSet {
    First,
    PreUpdate,
    Update,
    PostUpdate,
    Last,
}

pub(crate) mod prelude {
    pub use super::app::{App, Tick};
}