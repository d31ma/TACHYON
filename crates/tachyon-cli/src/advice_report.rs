use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

type AdvicePass = Box<dyn FnOnce() + Send>;

#[derive(Clone)]
pub(crate) struct AdviceReport {
    pending: Arc<Mutex<Option<AdvicePass>>>,
}

impl AdviceReport {
    pub(crate) fn simpler_forms(project: &Path) -> Self {
        let project = project.to_path_buf();
        Self::new(Box::new(move || {
            eprint!("{}", tachyon_core::SimplerForms::report(&project));
        }))
    }

    pub(crate) fn none() -> Self {
        Self::new(Box::new(|| {}))
    }

    pub(crate) fn new(pass: AdvicePass) -> Self {
        Self {
            pending: Arc::new(Mutex::new(Some(pass))),
        }
    }

    pub(crate) fn start_once(&self) {
        let pass = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        Self::spawn(pass);
    }

    fn spawn(pass: Option<AdvicePass>) {
        if let Some(pass) = pass {
            let _detached = std::thread::spawn(pass);
        }
    }
}
