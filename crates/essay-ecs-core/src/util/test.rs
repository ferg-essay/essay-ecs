use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TestValues(Arc<Mutex<Vec<String>>>);

impl TestValues {
    pub fn new() -> Self {
        TestValues(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn push(&mut self, value: &str) {
        self.0.lock().unwrap().push(value.to_string());
    }

    pub fn take(&mut self) -> String {
        let vec = self.0.lock().unwrap().drain(..).collect::<Vec<String>>();

        vec.join(", ")
    }
}