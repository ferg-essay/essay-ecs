use essay_ecs_app::{App, Update};

pub fn main() {
    let mut app = App::new();

    app.add_system(Update, || println!("Hello"));

    app.update();
}