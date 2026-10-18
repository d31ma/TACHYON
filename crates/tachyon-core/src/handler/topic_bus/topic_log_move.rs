use super::TopicFolder;
use crate::Failure;
use crate::migrate::{MigrationFinding, MigrationStatus};
use cap_fs_ext::DirExt as _;
use cap_std::fs::{Dir, DirEntry};
use std::path::Path;

pub(crate) struct TopicLogMove;

impl TopicLogMove {
    const OLD_PARENT: &'static str = ".tachyon";
    const OLD_NAME: &'static str = "topics";
    const OLD_LOCATION: &'static str = ".tachyon/topics";
    const LOG_EXTENSION: &'static str = ".jsonl";
    const NOT_READ: &'static str = "Old topic logs in .tachyon/topics cannot be read; move them to the topic log folder by hand.";
    const NOT_MOVED: &'static str = "Old topic logs in .tachyon/topics cannot be moved; move them to the topic log folder by hand.";

    pub(crate) fn run(project: &Dir, folder: &TopicFolder) -> Result<(), Failure> {
        let old = Self::old_logs(project).map_err(TopicFolder::failure)?;
        old.map(|(old, names)| Self::move_from(project, old, &names, folder))
            .transpose()
            .map_err(TopicFolder::failure)?;
        Ok(())
    }

    pub(crate) fn migration_findings(root: &Path) -> Vec<MigrationFinding> {
        let found = Self::old_logs_at(root).is_some();
        found.then(Self::migration_finding).into_iter().collect()
    }

    fn old_logs_at(root: &Path) -> Option<(Dir, Vec<String>)> {
        let project = Dir::open_ambient_dir(root, cap_std::ambient_authority()).ok()?;
        Self::old_logs(&project).ok().flatten()
    }

    fn old_logs(project: &Dir) -> Result<Option<(Dir, Vec<String>)>, &'static str> {
        let old = project.open_dir_nofollow(Self::OLD_LOCATION).ok();
        let found = old.map(Self::with_log_names).transpose()?;
        Ok(found.filter(Self::has_logs))
    }

    fn with_log_names(old: Dir) -> Result<(Dir, Vec<String>), &'static str> {
        let names = Self::log_names(&old)?;
        Ok((old, names))
    }

    fn has_logs(found: &(Dir, Vec<String>)) -> bool {
        !found.1.is_empty()
    }

    fn log_names(directory: &Dir) -> Result<Vec<String>, &'static str> {
        let mut names = Vec::new();
        for entry in directory.entries().map_err(|_| Self::NOT_READ)? {
            names.extend(Self::log_name(&entry.map_err(|_| Self::NOT_READ)?));
        }
        Ok(names)
    }

    fn log_name(entry: &DirEntry) -> Option<String> {
        let name = entry.file_name().into_string().ok()?;
        let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
        (is_file && name.ends_with(Self::LOG_EXTENSION)).then_some(name)
    }

    fn move_from(
        project: &Dir,
        old: Dir,
        names: &[String],
        folder: &TopicFolder,
    ) -> Result<(), &'static str> {
        let new = folder.create()?;
        let waiting = Self::names_to_move(&new, names, folder)?;
        Self::move_each(&old, &new, &waiting)?;
        // Windows keeps a directory while a handle to it is open.
        drop(old);
        Self::finish(project, &waiting, folder);
        Ok(())
    }

    fn names_to_move(
        new: &Dir,
        names: &[String],
        folder: &TopicFolder,
    ) -> Result<Vec<String>, &'static str> {
        if Self::log_names(new)?.is_empty() {
            return Ok(names.to_vec());
        }
        eprintln!(
            "Topic logs stay in {} because {} already holds topic logs; merge them by hand.",
            Self::OLD_LOCATION,
            folder.location()
        );
        Ok(Vec::new())
    }

    fn move_each(old: &Dir, new: &Dir, names: &[String]) -> Result<(), &'static str> {
        for name in names {
            old.rename(name, new, name).map_err(|_| Self::NOT_MOVED)?;
        }
        Ok(())
    }

    fn finish(project: &Dir, moved: &[String], folder: &TopicFolder) {
        if moved.is_empty() {
            return;
        }
        let _kept_when_not_empty = project
            .open_dir_nofollow(Self::OLD_PARENT)
            .and_then(|parent| parent.remove_dir(Self::OLD_NAME));
        eprintln!(
            "Moved {} topic logs from {} to {}. .tachyon/ now holds only generated files and is safe to delete.",
            moved.len(),
            Self::OLD_LOCATION,
            folder.location()
        );
    }

    fn migration_finding() -> MigrationFinding {
        MigrationFinding {
            source: String::from(Self::OLD_LOCATION),
            feature: String::from("server.topic_logs"),
            status: MigrationStatus::Changed,
            detail: String::from(
                "Topic logs now live in the folder that topics.directory in server/application.json names (default data/topics). .tachyon/ holds only generated files.",
            ),
            action: Some(String::from(
                "Start the server once: it moves these logs to the topic log folder and prints a notice. Or move them by hand.",
            )),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::TopicFolder;
    use super::TopicLogMove;
    use cap_std::fs::Dir;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    fn run(root: &Path) {
        let project =
            Arc::new(Dir::open_ambient_dir(root, cap_std::ambient_authority()).expect("project"));
        let folder = TopicFolder::resolve(Arc::clone(&project), None).expect("folder");
        TopicLogMove::run(&project, &folder).expect("moved");
    }

    #[test]
    fn old_logs_move_to_the_topic_folder_once() {
        let root = tempfile::tempdir().expect("root");
        let old = root.path().join(".tachyon/topics");
        fs::create_dir_all(&old).expect("old folder");
        fs::write(old.join("order-placed.jsonl"), "{\"id\":1}\n").expect("old log");
        assert_eq!(TopicLogMove::migration_findings(root.path()).len(), 1);
        run(root.path());
        let moved = fs::read_to_string(root.path().join("data/topics/order-placed.jsonl"))
            .expect("moved log");
        assert_eq!(moved, "{\"id\":1}\n");
        assert!(!old.exists());
        assert!(TopicLogMove::migration_findings(root.path()).is_empty());
        fs::create_dir_all(&old).expect("old folder again");
        fs::write(old.join("order-placed.jsonl"), "{\"id\":2}\n").expect("second old log");
        run(root.path());
        let kept = fs::read_to_string(root.path().join("data/topics/order-placed.jsonl"))
            .expect("kept log");
        assert_eq!(kept, "{\"id\":1}\n");
        assert!(old.join("order-placed.jsonl").is_file());
    }

    #[test]
    fn a_project_without_old_logs_gets_no_folder() {
        let root = tempfile::tempdir().expect("root");
        run(root.path());
        assert!(!root.path().join("data").exists());
        assert!(TopicLogMove::migration_findings(root.path()).is_empty());
    }
}
