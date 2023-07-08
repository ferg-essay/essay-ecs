use std::sync::{Arc, Mutex};

use essay_ecs_core::core_app::CoreApp;

#[test]
fn test_hello() {
    let mut app = CoreApp::new();

    let vec = Vec::<String>::new();
    let arc = Arc::new(Mutex::new(vec));

    let ptr = arc.clone();

    app.add_system(move || ptr.lock().unwrap().push("hello, world".into()));

    assert_eq!(take(&arc), "");
}

fn take(arc: &Arc<Mutex<Vec<String>>>) -> String {
    let vec : Vec<String> = arc.lock().unwrap().drain(..).collect();

    vec.join(", ")
}