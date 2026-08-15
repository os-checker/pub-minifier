mod api {
    pub fn free_fn() {}

    pub struct Worker;

    impl Worker {
        pub fn new() -> Self {
            Worker
        }

        pub fn run(&self) {
            free_fn();
        }
    }

    pub mod nested {
        pub fn ping() {}
    }
}

fn main() {
    api::free_fn();
    api::nested::ping();

    let w = api::Worker::new();
    w.run();
}
