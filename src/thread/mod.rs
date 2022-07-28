enum ThreadStatus {
    Running,
    Stopped,
}

struct Thread {
    status: ThreadStatus,
    scripts: Vec<Box<dyn Fn()>>,
}
