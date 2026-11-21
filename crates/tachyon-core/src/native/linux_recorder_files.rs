pub(super) struct LinuxRecorderFiles;

impl LinuxRecorderFiles {
    pub(super) const SOURCE: &'static str = include_str!("linux_recorder_files.c");
}
