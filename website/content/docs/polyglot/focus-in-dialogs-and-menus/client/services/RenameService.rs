#[Service]
#[derive(Default)]
struct RenameService {
    focus: Focus,
    editing: bool,
    problem: String,
}

impl RenameService {
    pub fn open(&mut self) {
        self.editing = true;
        let trapped = self.focus.trap("dialog");
        self.report(trapped);
        let moved = self.focus.move_to("name");
        self.report(moved);
    }

    pub fn close(&mut self) {
        self.editing = false;
        let released = self.focus.release();
        self.report(released);
    }

    fn report(&mut self, answer: Result<(), String>) {
        if let Err(message) = answer {
            self.problem = message;
        }
    }

    #[OnEvent]
    fn failed(&mut self, failed: BrowserCallFailed) {
        self.problem = failed.message;
    }
}
