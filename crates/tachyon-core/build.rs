use std::fs;
use std::io;
use std::path::{Path, PathBuf};

struct TemplateIndex;

impl TemplateIndex {
    const ROOT: &'static str = "src/scaffold/templates";
    const SUFFIX: &'static str = ".tmpl";
    const OUTPUT: &'static str = "scaffold_templates.rs";

    fn write() -> io::Result<()> {
        println!("cargo::rerun-if-changed={}", Self::ROOT);
        let root = Self::manifest_folder()?.join(Self::ROOT);
        let mut files = Vec::new();
        Self::collect(&root, &mut files)?;
        files.sort();
        let rows: Vec<String> = files.iter().map(|path| Self::row(&root, path)).collect();
        let text = format!("&[\n{}]\n", rows.concat());
        fs::write(Self::output_file()?, text)
    }

    fn collect(folder: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(folder)? {
            Self::add(&entry?.path(), files)?;
        }
        Ok(())
    }

    fn add(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        if path.is_dir() {
            return Self::collect(path, files);
        }
        files.push(path.to_path_buf());
        Ok(())
    }

    fn row(root: &Path, path: &Path) -> String {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let name = relative.to_string_lossy().replace('\\', "/");
        let name = name.trim_end_matches(Self::SUFFIX);
        let absolute = path.to_string_lossy();
        format!("    ({name:?}, include_str!({absolute:?})),\n")
    }

    fn manifest_folder() -> io::Result<PathBuf> {
        Self::variable("CARGO_MANIFEST_DIR")
    }

    fn output_file() -> io::Result<PathBuf> {
        Ok(Self::variable("OUT_DIR")?.join(Self::OUTPUT))
    }

    fn variable(name: &str) -> io::Result<PathBuf> {
        std::env::var_os(name)
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other(format!("Cargo did not set {name}.")))
    }
}

fn main() -> io::Result<()> {
    TemplateIndex::write()
}
