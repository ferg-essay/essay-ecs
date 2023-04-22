mod params;
pub mod base_app;
mod util;
pub mod schedule;
pub mod entity;
mod systems;
mod world;

pub use essay_ecs_macros::{
    Component, ScheduleLabel, Phase
};

pub use schedule::{
    IntoSystem, IntoSystemConfig,
    IntoPhaseConfig, IntoPhaseConfigs,
    Schedule, Schedules,
};

pub use params::{
    Local,
    Commands, Res, ResMut, 
};

pub use world::{
    World
};

pub mod prelude {
    pub use crate::world::{World};
    pub use crate::params::{Commands, Param, Res, ResMut};
    pub use essay_ecs_macros::{Component, ScheduleLabel, Phase};

    pub use crate::schedule::{
        IntoSystem, IntoSystemConfig,
        IntoPhaseConfig, IntoPhaseConfigs,
    };
}
