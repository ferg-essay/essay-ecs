pub mod error;
pub mod param;
pub mod core_app;
pub mod util;
pub mod schedule;
pub mod entity;
pub mod system;
pub mod store;
pub mod resource;

pub use essay_ecs_core_macros::{
    Component, ScheduleLabel, Param, Phase
};

pub use schedule::{
    IntoPhaseConfig, IntoPhaseConfigs,
    Schedule, Schedules, IntoSystemConfig,
};

pub use system::IntoSystem;

pub use param::{
    Local, Res, ResMut, Query, 
};

pub use store::{
    Store, Commands
};

pub mod prelude {
    pub use essay_ecs_core_macros::{Component, ScheduleLabel, Param, Phase};

    pub use crate::{
        param::{Res, ResMut, Local, Query},

        schedule::{
            IntoPhaseConfig, IntoPhaseConfigs, IntoSystemConfig,
        },

        system::IntoSystem, 

        store::{Commands, FromStore},
    };
}
