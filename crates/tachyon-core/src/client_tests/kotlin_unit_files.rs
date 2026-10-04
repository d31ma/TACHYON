use std::path::{Path, PathBuf};

pub(super) struct KotlinUnitFiles;

impl KotlinUnitFiles {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "BehaviorTasks.kt",
            concat!(
                "import kotlin.coroutines.startCoroutine\n\n",
                include_str!("kotlin_test_tasks.kt")
            ),
        ),
        (
            "TacReads.kt",
            concat!(
                include_str!("../native/prelude_reads.kt"),
                include_str!("../native/prelude_jvm_models.kt")
            ),
        ),
    ];

    pub(super) fn write(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
        Self::FILES
            .iter()
            .map(|(name, text)| {
                let path = directory.join(name);
                std::fs::write(&path, text).map(|()| path)
            })
            .collect()
    }
}
