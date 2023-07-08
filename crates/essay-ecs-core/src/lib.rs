mod param;
pub mod core_app;
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
    Res, ResMut, Query, 
};

pub use world::{
    World, Commands
};

pub mod prelude {
    pub use crate::world::{World};
    pub use crate::param::{Param, Res, ResMut};
    pub use essay_ecs_macros::{Component, ScheduleLabel, Phase};

    pub use crate::schedule::{
        IntoPhaseConfig, IntoPhaseConfigs,
    };

    pub use crate::system::{
        IntoSystem, IntoSystemConfig,
    };
}
