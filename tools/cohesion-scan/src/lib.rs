mod file_name;
mod function;
mod function_scanner;
mod parsed_file;
mod rust_tokens;
mod source_files;
mod structure;

use file_name::FileName;
use function::Function;
use function_scanner::FunctionScanner;
use parsed_file::ParsedFile;
use serde::Serialize;
use source_files::SourceFiles;
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

pub use function::{Construct, Found, Violation};
pub use structure::Structure;

#[derive(Debug)]
pub struct ScanError {
    path: PathBuf,
    message: String,
}

impl ScanError {
    fn new(path: &Path, error: &dyn fmt::Display) -> Self {
        Self {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

impl std::error::Error for ScanError {}

#[derive(Debug, Serialize)]
pub struct FileReport {
    pub path: String,
    pub violations: Vec<Violation>,
    pub structure: Structure,
    pub wanted_file_name: Option<String>,
}

impl FileReport {
    fn new(path: String, functions: Vec<Function>, structure: Structure) -> Self {
        let wanted_file_name = FileName::wanted(&path, &structure);
        let branching_helpers: BTreeSet<String> = functions
            .iter()
            .filter_map(Function::branching_helper_name)
            .collect();
        let violations = functions
            .into_iter()
            .filter_map(|function| function.verdict(&branching_helpers))
            .collect();
        Self {
            path,
            violations,
            structure,
            wanted_file_name,
        }
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.violations.len()
    }

    #[must_use]
    pub fn public_total(&self) -> usize {
        self.violations
            .iter()
            .filter(|violation| violation.public)
            .count()
    }

    #[must_use]
    pub fn private_total(&self) -> usize {
        self.total() - self.public_total()
    }

    #[must_use]
    pub fn structure_total(&self) -> usize {
        self.structure.total()
    }

    #[must_use]
    pub fn file_name_total(&self) -> usize {
        usize::from(self.wanted_file_name.is_some())
    }

    fn has_violations(&self) -> bool {
        self.total() + self.structure_total() + self.file_name_total() > 0
    }

    fn row(&self) -> String {
        Report::format_row(
            [
                self.total(),
                self.public_total(),
                self.private_total(),
                self.structure_total(),
                self.file_name_total(),
            ],
            &self.path,
        )
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub total: usize,
    pub public_total: usize,
    pub private_total: usize,
    pub structure_total: usize,
    pub file_name_total: usize,
    pub duplication_total: usize,
    pub files: Vec<FileReport>,
    pub duplicates: Vec<String>,
}

impl Report {
    fn new(files: Vec<FileReport>, duplicates: Vec<String>) -> Self {
        let files: Vec<FileReport> = files
            .into_iter()
            .filter(FileReport::has_violations)
            .collect();
        Self {
            total: files.iter().map(FileReport::total).sum(),
            public_total: files.iter().map(FileReport::public_total).sum(),
            private_total: files.iter().map(FileReport::private_total).sum(),
            structure_total: files.iter().map(FileReport::structure_total).sum(),
            file_name_total: files.iter().map(FileReport::file_name_total).sum(),
            duplication_total: duplicates.len(),
            files,
            duplicates,
        }
    }

    fn format_row<T: fmt::Display>(counts: [T; 5], name: &str) -> String {
        let [total, public, private, structure, file_name] = counts;
        format!("{total:>6} {public:>6} {private:>7} {structure:>9} {file_name:>9}  {name}")
    }

    pub fn json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    #[must_use]
    pub fn table(&self) -> String {
        let header = Self::format_row(
            ["total", "public", "private", "structure", "file-name"],
            "file",
        );
        let rows: Vec<String> = self.files.iter().map(FileReport::row).collect();
        let footer = Self::format_row(
            [
                self.total,
                self.public_total,
                self.private_total,
                self.structure_total,
                self.file_name_total,
            ],
            "TOTAL",
        );
        let duplicates = self.duplicates.join("\n");
        let duplication = format!("{:>6} duplicated blocks", self.duplication_total);
        format!(
            "{header}\n{}\n{footer}\n{duplicates}\n{duplication}",
            rows.join("\n")
        )
    }
}

pub struct Scan;

impl Scan {
    pub fn repository(root: &Path) -> Result<Report, ScanError> {
        let paths = SourceFiles::under(root)?;
        let parsed = paths
            .iter()
            .map(|path| ParsedFile::read(root, path))
            .collect::<Result<Vec<ParsedFile>, ScanError>>()?;
        let files = parsed.iter().map(Self::file).collect();
        Ok(Report::new(files, ParsedFile::duplicates(&parsed)))
    }

    fn file(parsed: &ParsedFile) -> FileReport {
        FileReport::new(
            parsed.relative.clone(),
            FunctionScanner::functions(&parsed.syntax),
            Structure::of(&parsed.syntax),
        )
    }
}
