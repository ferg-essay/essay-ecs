use essay_ecs_app::app::App;
use essay_ecs_core::core_app::{Core};

pub fn main() {
    let mut app = App::new();

    app.add_system(Core, || println!("Hello"));

    app.update();
}