use command_group::AsyncGroupChild;
use std::path::PathBuf;

#[derive(Debug)]
pub(super) enum NativeRunningApp {
    Desktop(Box<AsyncGroupChild>),
    Simulator { device: String, identifier: String },
    Emulator { adb: PathBuf, package: String },
}
