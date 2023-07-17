pub mod event;
mod channel;
mod app;

pub use crate::app::*;

pub mod prelude {
    pub use crate::{
        app::{
            App,
            PreStartup, Startup, PostStartup,
            Main, 
            First, PreUpdate, Update, PostUpdate, Last,

            Plugin, 
        },
        event::{Events, InEvent, OutEvent},
    };

    pub use essay_ecs_app_macros::Event;
}