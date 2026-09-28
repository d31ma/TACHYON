use super::TopicLogMove;
use crate::failure::diagnostic;
use crate::{ApplicationConfig, Failure};
use cap_fs_ext::DirExt as _;
use cap_std::fs::Dir;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct TopicFolder {
    parent: Arc<Dir>,
    name: PathBuf,
    location: String,
}

impl std::fmt::Debug for TopicFolder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.location)
    }
}

impl TopicFolder {
    pub(crate) const DEFAULT: &'static str = "data/topics";
    const GENERATED: &'static str = ".tachyon";
    const DIAGNOSTIC: u16 = 1303;
    const HELP: &'static str = "Set topics.directory in server/application.json to a plain folder path inside the project, such as data/topics, or to an absolute path with no symbolic link in it. It must not be inside .tachyon/, which holds only generated files.";
    const NOT_PLAIN: &'static str =
        "topics.directory must be a plain folder path with no '..', '.', or empty part.";
    const INSIDE_GENERATED: &'static str =
        "topics.directory must not be inside .tachyon/, which holds only generated files.";
    const LINKED: &'static str =
        "An absolute topics.directory must not pass through a symbolic link.";
    const NOT_CREATED: &'static str = "The topic log folder cannot be created.";
    const NOT_OPENED: &'static str = "The topic log folder cannot be opened.";

    pub(crate) fn prepare(
        project: &Arc<Dir>,
        application: &ApplicationConfig,
    ) -> Result<Self, Failure> {
        let folder = Self::resolve(Arc::clone(project), application.topics_directory())?;
        TopicLogMove::run(project, &folder)?;
        Ok(folder)
    }

    pub(crate) fn resolve(project: Arc<Dir>, configured: Option<&str>) -> Result<Self, Failure> {
        let path = Path::new(configured.unwrap_or(Self::DEFAULT));
        Self::from_path(project, path).map_err(Self::failure)
    }

    pub(crate) fn location(&self) -> &str {
        &self.location
    }

    pub(crate) fn open(&self) -> Result<Option<Dir>, &'static str> {
        Self::found(self.parent.open_dir_nofollow(&self.name))
    }

    pub(crate) fn create(&self) -> Result<Dir, &'static str> {
        self.parent
            .create_dir_all(&self.name)
            .map_err(|_| Self::NOT_CREATED)?;
        self.parent
            .open_dir_nofollow(&self.name)
            .map_err(|_| Self::NOT_OPENED)
    }

    fn from_path(project: Arc<Dir>, path: &Path) -> Result<Self, &'static str> {
        if path.is_absolute() {
            return Self::outside(path);
        }
        Self::inside(project, path)
    }

    fn inside(project: Arc<Dir>, path: &Path) -> Result<Self, &'static str> {
        let parts = Self::plain_parts(path)?;
        Self::refuse_generated(&parts)?;
        Ok(Self {
            parent: project,
            name: parts.iter().collect(),
            location: parts.join("/"),
        })
    }

    fn outside(path: &Path) -> Result<Self, &'static str> {
        Self::refuse_parent_steps(path)?;
        Self::refuse_links(path)?;
        std::fs::create_dir_all(path).map_err(|_| Self::NOT_CREATED)?;
        let parent = path.parent().ok_or(Self::NOT_PLAIN)?;
        let name = path.file_name().ok_or(Self::NOT_PLAIN)?;
        let parent = Dir::open_ambient_dir(parent, cap_std::ambient_authority())
            .map_err(|_| Self::NOT_OPENED)?;
        Ok(Self {
            parent: Arc::new(parent),
            name: PathBuf::from(name),
            location: path.display().to_string(),
        })
    }

    fn plain_parts(path: &Path) -> Result<Vec<String>, &'static str> {
        let parts = path
            .components()
            .map(Self::plain_part)
            .collect::<Result<Vec<String>, &'static str>>()?;
        Self::refuse_empty(parts)
    }

    fn plain_part(component: Component<'_>) -> Result<String, &'static str> {
        match component {
            Component::Normal(part) => part.to_str().map(String::from).ok_or(Self::NOT_PLAIN),
            _ => Err(Self::NOT_PLAIN),
        }
    }

    fn refuse_empty(parts: Vec<String>) -> Result<Vec<String>, &'static str> {
        if parts.is_empty() {
            return Err(Self::NOT_PLAIN);
        }
        Ok(parts)
    }

    fn refuse_parent_steps(path: &Path) -> Result<(), &'static str> {
        if path.components().any(Self::is_parent_step) {
            return Err(Self::NOT_PLAIN);
        }
        Ok(())
    }

    fn is_parent_step(part: Component<'_>) -> bool {
        part == Component::ParentDir
    }

    fn refuse_generated(parts: &[String]) -> Result<(), &'static str> {
        let first = parts.first().map_or("", String::as_str);
        if first.eq_ignore_ascii_case(Self::GENERATED) {
            return Err(Self::INSIDE_GENERATED);
        }
        Ok(())
    }

    fn refuse_links(path: &Path) -> Result<(), &'static str> {
        if path.ancestors().any(Self::is_link) {
            return Err(Self::LINKED);
        }
        Ok(())
    }

    fn is_link(path: &Path) -> bool {
        std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
    }

    fn found(opened: std::io::Result<Dir>) -> Result<Option<Dir>, &'static str> {
        match opened {
            Ok(directory) => Ok(Some(directory)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(Self::NOT_OPENED),
        }
    }

    pub(crate) fn failure(message: &'static str) -> Failure {
        Failure::one(diagnostic(
            Self::DIAGNOSTIC,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::TopicFolder;
    use cap_std::fs::Dir;
    use std::path::Path;
    use std::sync::Arc;

    fn project(root: &Path) -> Arc<Dir> {
        Arc::new(Dir::open_ambient_dir(root, cap_std::ambient_authority()).expect("project"))
    }

    #[test]
    fn the_default_folder_is_data_topics_outside_the_generated_folder() {
        let root = tempfile::tempdir().expect("root");
        let folder = TopicFolder::resolve(project(root.path()), None).expect("default");
        assert_eq!(folder.location(), "data/topics");
        assert!(folder.open().expect("missing is fine").is_none());
        folder.create().expect("created");
        assert!(root.path().join("data/topics").is_dir());
        assert!(!root.path().join(".tachyon").exists());
    }

    #[test]
    fn a_plain_project_path_is_used() {
        let root = tempfile::tempdir().expect("root");
        let folder =
            TopicFolder::resolve(project(root.path()), Some("storage/events")).expect("plain");
        folder.create().expect("created");
        assert!(root.path().join("storage/events").is_dir());
    }

    #[test]
    fn unsafe_paths_are_refused() {
        let root = tempfile::tempdir().expect("root");
        for path in [
            "",
            "../outside",
            "data/../../outside",
            "./data",
            ".tachyon/topics",
            ".Tachyon/topics",
        ] {
            assert!(
                TopicFolder::resolve(project(root.path()), Some(path)).is_err(),
                "{path}"
            );
        }
        assert!(!root.path().join(".tachyon").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_folder_cannot_lead_out_of_the_project() {
        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), root.path().join("data")).expect("link");
        let folder = TopicFolder::resolve(project(root.path()), None).expect("resolved");
        assert!(folder.create().is_err());
        assert!(folder.open().is_err());
        assert!(
            std::fs::read_dir(outside.path())
                .expect("read")
                .next()
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_absolute_path_is_used_only_without_links() {
        let root = tempfile::tempdir().expect("root");
        let place = tempfile::tempdir().expect("place");
        let real = std::fs::canonicalize(place.path()).expect("real place");
        let linked = root.path().join("linked");
        std::os::unix::fs::symlink(&real, &linked).expect("link");
        let through_link = linked.join("topics");
        let refused = TopicFolder::resolve(project(root.path()), through_link.to_str());
        assert!(refused.is_err());
        assert!(!real.join("topics").exists());
        let direct = real.join("topics");
        let folder = TopicFolder::resolve(project(root.path()), direct.to_str()).expect("absolute");
        folder.create().expect("created");
        assert!(folder.open().expect("opened").is_some());
        let parent_step = real.join("x/../topics");
        assert!(TopicFolder::resolve(project(root.path()), parent_step.to_str()).is_err());
    }
}
