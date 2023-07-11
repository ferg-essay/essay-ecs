use essay_ecs_app::{App, Update};

pub fn main() {
    let mut app = App::new();

    app.system(Update, || println!("Hello"));

    app.tick();
}