#[Service]
#[derive(Default)]
struct ReportService {
    window_controls: WindowControls,
    problem: String,
}

impl ReportService {
    pub fn minimize(&mut self) {
        self.problem = self.window_controls.minimize().err().unwrap_or_default();
    }

    pub fn resize(&mut self) {
        self.problem = self.window_controls.resize(1280, 800).err().unwrap_or_default();
    }

    pub fn rename(&mut self) {
        self.problem = self.window_controls.set_title("Quarterly report").err().unwrap_or_default();
    }
}
