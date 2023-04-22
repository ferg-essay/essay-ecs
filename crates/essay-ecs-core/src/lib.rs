mod param;
pub mod base_app;
mod util;
pub mod schedule;
pub mod entity;
pub mod system;
mod world;
mod resource;

pub use essay_ecs_macros::{
    Component, ScheduleLabel, Phase
};

pub use schedule::{
    IntoPhaseConfig, IntoPhaseConfigs,
    Schedule, Schedules,
};

pub use system::{
    IntoSystem, IntoSystemConfig,
};

pub use param::{
    Local,
    Commands, Res, ResMut, Query, 
};

pub use world::{
    World
};

pub mod prelude {
    pub use crate::world::{World};
    pub use crate::param::{Commands, Param, Res, ResMut};
    pub use essay_ecs_macros::{Component, ScheduleLabel, Phase};

    pub use crate::schedule::{
        IntoPhaseConfig, IntoPhaseConfigs,
    };

    pub use crate::system::{
        IntoSystem, IntoSystemConfig,
    };
}
