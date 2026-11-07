use std::fs::{self, Metadata};
use std::path::Path;

pub(crate) struct ToolBuild;

impl ToolBuild {
    pub(crate) fn id(executable: Option<&Path>) -> String {
        let mut id = String::from(env!("CARGO_PKG_VERSION"));
        let metadata = executable.and_then(|path| fs::metadata(path).ok());
        id.push_str(
            &metadata
                .map(|metadata| Self::build_suffix(&metadata))
                .unwrap_or_default(),
        );
        id
    }

    fn build_suffix(metadata: &Metadata) -> String {
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_nanos());
        format!("+{}.{modified}", metadata.len())
    }
}
