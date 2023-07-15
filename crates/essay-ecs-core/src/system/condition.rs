#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use crate::core_app::{CoreApp, Core};
    use crate::IntoSystemConfig;

    #[test]
    fn run_if() {
        let mut app = CoreApp::new();

        let values = Arc::new(Mutex::new(Vec::<String>::new()));
        
        let ptr = values.clone();
        app.system(Core, (move || { push(&ptr, "system-true" ); })
            .run_if(run_true)
        );
        
        let ptr = values.clone();
        app.system(Core, (move || { push(&ptr, "system-false" ); })
            .run_if(run_false)
        );

        app.tick();
        assert_eq!(take(&values), "system-true");

        app.tick();
        assert_eq!(take(&values), "system-true");
    }

    fn push(ptr: &Arc<Mutex<Vec<String>>>, value: &str) {
        ptr.lock().unwrap().push(value.to_string());
    }

    fn take(ptr: &Arc<Mutex<Vec<String>>>) -> String {
        let values : Vec<String> = ptr.lock().unwrap().drain(..).collect();

        values.join(",")
    }

    fn run_true() -> bool {
        true
    }

    fn run_false() -> bool {
        false
    }
}