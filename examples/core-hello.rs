use essay_ecs_core::core_app::{CoreApp, Core};

///
/// Hello, world for essay-ecs-core.
/// 
/// The core contains the base capabilities needed for the ecs to
/// work.
/// 
fn main() {
    let mut app = CoreApp::new();

    app.system(Core, || println!("Hello, world") );

    // evaluate all systems in the application
    app.tick().unwrap();
    app.tick().unwrap();
}