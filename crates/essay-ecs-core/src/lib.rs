pub mod param;
pub mod core_app;
pub mod util;
pub mod schedule;
pub mod entity;
pub mod system;
pub mod world;
pub mod resource;

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
    pub use essay_ecs_macros::{Component, ScheduleLabel, Phase};

    pub use crate::{
        param::{Param, Res, ResMut},

        schedule::{
            IntoPhaseConfig, IntoPhaseConfigs,
        },

        system::{
            IntoSystem, IntoSystemConfig,
        },

        world::{
            World, Commands,
        }
    };
}
