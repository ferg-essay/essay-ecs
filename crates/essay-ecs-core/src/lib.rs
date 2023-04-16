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

pub use systems::{
    Local
};

pub use world::{
    Commands, Res, ResMut, 
    World
};

pub mod prelude {
    pub use crate::world::{Commands, World, Res, ResMut};
    pub use essay_ecs_macros::{Component, ScheduleLabel, Phase};
    pub use crate::systems::{
        Param, Local
    };

    pub use crate::schedule::{
        IntoSystem, IntoSystemConfig,
        IntoPhaseConfig, IntoPhaseConfigs,
    };
}
