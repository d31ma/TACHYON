use super::CompilerErrorLine;

#[test]
fn go_causal_locations_precede_the_too_many_errors_summary() {
    for cause in [
        "./server/shared/mailbox/DueFollowUps.go:11: undefined: FollowUpRemindersHelper",
        "./handler.go:4:8: cannot use value (variable of type string) as int value",
        "./handler.go:5: Name redeclared in this block",
        "./handler.go:8: cannot use \"too many errors\" (untyped string constant) as int value",
        "./handler.go:4: undefined: warning",
        "./handler.go:4: undefined: note",
        "./handler.go:4: undefined: help",
        "./handler.go:8: cannot use \"message: too many errors\" (untyped string constant) as int value",
    ] {
        let detail = format!(
            "# yonhandler\n{cause}\n./server/shared/mailbox/DueFollowUps.go:13: too many errors\n"
        );
        assert_eq!(CompilerErrorLine::read("go", &detail), cause);
    }
}

#[test]
fn actual_compiler_severity_precedes_warning_text_that_mentions_error() {
    for (program, cause) in [
        ("rustc", "error[E0432]: unresolved import"),
        (
            "csc",
            "handler.cs(3,4): error CS0103: The name does not exist",
        ),
        ("javac", "Handler.java:3: error: cannot find symbol"),
        ("g++", "handler.cpp:2: fatal error: missing header"),
        ("dart", "handler.dart:2:3: Error: Method not found"),
    ] {
        let detail = format!(
            "compiler banner\nwarning: unused error_provider\nwarning: error: historical failure\nnote: error: additional context\n{cause}\nerror: aborting due to previous errors\n"
        );
        assert_eq!(CompilerErrorLine::read(program, &detail), cause);
    }
}

#[test]
fn compiler_output_keeps_the_existing_content_fallback() {
    for (detail, expected) in [
        ("", "no detail"),
        ("\n# yonhandler\n", "no detail"),
        ("# yonhandler\nopaque failure\n", "opaque failure"),
        (
            "# yonhandler\nhandler.go:not-a-line: message\n",
            "handler.go:not-a-line: message",
        ),
        (
            "# yonhandler\nhandler.go:4: too many errors\n",
            "handler.go:4: too many errors",
        ),
    ] {
        assert_eq!(CompilerErrorLine::read("go", detail), expected);
    }
}
