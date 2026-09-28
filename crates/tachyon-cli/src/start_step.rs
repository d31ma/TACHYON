use tachyon_core::{DevServer, ErrorPageServer};

pub(crate) enum StartStep {
    Retry(Option<Box<ErrorPageServer>>),
    Serve(Box<DevServer>),
    Stopped,
}
