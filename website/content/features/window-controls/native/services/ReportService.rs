#[Service]
#[NeedsAll]
#[derive(Default)]
struct ReportService {
    window_controls: WindowControls,
    problem: String,
}

impl ReportService {
    fn minimize(&mut self) {
        self.problem = self.window_controls.minimize().err().unwrap_or_default();
    }

    fn resize(&mut self) {
        self.problem = self.window_controls.resize(1280, 800).err().unwrap_or_default();
    }

    fn rename(&mut self) {
        self.problem = self.window_controls.set_title("Quarterly report").err().unwrap_or_default();
    }
}
