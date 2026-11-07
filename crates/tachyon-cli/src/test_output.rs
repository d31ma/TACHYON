#[derive(Clone, Copy)]
pub(crate) enum TestOutput {
    Terminal,
    Quiet,
}

impl TestOutput {
    pub(crate) fn line(self, line: &str) {
        self.write(line);
    }

    pub(crate) fn failure(self, line: &str) {
        self.write_failure(line);
    }

    fn write(self, line: &str) {
        if matches!(self, Self::Terminal) {
            println!("{line}");
        }
    }

    fn write_failure(self, line: &str) {
        match self {
            Self::Terminal => println!("{line}"),
            Self::Quiet => eprintln!("{line}"),
        }
    }
}
