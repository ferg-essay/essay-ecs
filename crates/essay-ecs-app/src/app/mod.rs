mod main_schedule;

use essay_ecs_core::prelude::{Phase};

mod plugin;
mod app;

#[derive(Phase, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreTaskSet {
    First,
    PreUpdate,
    Update,
    PostUpdate,
    Last,
}

pub use app::App;

pub use main_schedule::{
    Main, Startup, Update, 
    MainSchedulePlugin,
};

pub use plugin::{
    Plugin,
};


