#[derive(Clone, Copy, Debug)]
pub struct TacTaskError {
    pub code: &'static str,
    pub message: &'static str,
}

impl TacTaskError {
    pub fn busy() -> Self {
        Self {
            code: "BUSY",
            message: "The service is still running another task.",
        }
    }
}
