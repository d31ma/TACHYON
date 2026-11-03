use super::super::duplication::ProjectSource;
use super::design_problem::DesignProblem;
use test_case::TestCase;
use test_file::TestFile;

mod test_case;
mod test_file;
mod test_language;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(super) struct TestRule;

impl TestRule {
    pub(super) fn problems(sources: &[ProjectSource]) -> Vec<DesignProblem> {
        let files: Vec<TestFile> = sources.iter().filter_map(TestFile::read).collect();
        files.iter().flat_map(Self::file_problems).collect()
    }

    fn file_problems(file: &TestFile) -> Vec<DesignProblem> {
        TestCase::cases(file)
            .iter()
            .flat_map(TestCase::problems)
            .collect()
    }
}
