pub(super) struct TestUnitError;

impl TestUnitError {
    const SUMMARY_BYTES: usize = 2048;
    const SUMMARY_LINES: usize = 6;

    pub(super) fn summary(stderr: &[u8], stdout: &[u8]) -> String {
        Self::bounded(&Self::lines(Self::selected(stderr, stdout)))
    }

    fn selected<'a>(stderr: &'a [u8], stdout: &'a [u8]) -> &'a [u8] {
        if stderr.is_empty() { stdout } else { stderr }
    }

    fn lines(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes)
            .lines()
            .take(Self::SUMMARY_LINES)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn bounded(text: &str) -> String {
        let end = text
            .char_indices()
            .map(|(index, character)| index + character.len_utf8())
            .take_while(|end| *end <= Self::SUMMARY_BYTES)
            .last()
            .unwrap_or(0);
        text[..end].to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::TestUnitError;

    #[test]
    fn stdout_is_used_only_when_stderr_is_empty() {
        assert_eq!(
            TestUnitError::summary(b"", b"compiler: invalid syntax\nsecond line\n"),
            "compiler: invalid syntax second line"
        );
        assert_eq!(
            TestUnitError::summary(b"stderr diagnostic\n", b"stdout diagnostic\n"),
            "stderr diagnostic"
        );
        assert_eq!(TestUnitError::summary(b" ", b"stdout diagnostic\n"), " ");
        assert_eq!(TestUnitError::summary(b"", b""), "");
    }

    #[test]
    fn diagnostics_keep_six_lines_and_bound_utf8_bytes() {
        assert_eq!(
            TestUnitError::summary(b"", b"one\ntwo\nthree\nfour\nfive\nsix\nseven"),
            "one two three four five six"
        );
        let text = "雪".repeat(1024);
        let summary = TestUnitError::summary(b"", text.as_bytes());
        assert_eq!(summary.len(), 2046);
        assert!(summary.chars().all(|value| value == '雪'));
        assert_eq!(TestUnitError::summary(b"", &[255]), "�");
    }

    #[cfg(unix)]
    #[test]
    fn failed_test_tool_retains_stdout_diagnostic_and_authored_path() {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "printf 'test compiler: invalid syntax\n'; exit 1"]);
        let outcome = crate::client_tests::execute(&mut command, "client/tests/ToolServiceTest.cs");
        let error = match outcome {
            Ok(_) => panic!("controlled compiler fixture must fail"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("The test unit did not run: test compiler: invalid syntax"));
        assert!(error.contains("client/tests/ToolServiceTest.cs"));
    }
}
