struct TacTaskWaker {
    task: u32,
}

impl std::task::Wake for TacTaskWaker {
    fn wake(self: std::sync::Arc<Self>) {
        TacTasks::wake(self.task);
    }
}
