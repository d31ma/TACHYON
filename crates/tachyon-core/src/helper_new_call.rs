pub(crate) struct HelperNewCall;

impl HelperNewCall {
    pub(crate) fn matches(before: &str, after: &str) -> bool {
        Self::new_word_then_parenthesis(before.split_whitespace().next_back(), after.trim_start())
    }

    fn new_word_then_parenthesis(word: Option<&str>, tail: &str) -> bool {
        word == Some("new") && tail.starts_with('(')
    }
}

#[cfg(test)]
mod tests {
    use super::HelperNewCall;

    fn called(code: &str, name: &str) -> bool {
        code.match_indices(name)
            .any(|(at, _)| HelperNewCall::matches(&code[..at], &code[at + name.len()..]))
    }

    #[test]
    fn explicit_new_call_needs_the_new_word_and_an_open_parenthesis() {
        for code in [
            "var value = new ProbeDelegate(context);",
            "let value = new ProbeDelegate ()",
            "let value =\n    new\n        ProbeDelegate\n        (context)",
            "return new ProbeDelegate(context);",
        ] {
            assert!(called(code, "ProbeDelegate"), "{code}");
        }
        for code in [
            "var value = ProbeDelegate(context);",
            "var value = renew ProbeDelegate(context);",
            "var value = x.new ProbeDelegate(context);",
            "var value = new ProbeDelegate<int>(context);",
            "var value = new ProbeDelegate.Inner(context);",
            "var value = new ProbeDelegate { };",
            "var value = new ProbeDelegate",
            "new ProbeDelegateSuffix(context)",
            "Type kind = typeof(ProbeDelegate);",
        ] {
            assert!(!called(code, "ProbeDelegate"), "{code}");
        }
    }

    #[test]
    fn split_word_and_tail_give_the_same_answer_as_raw_text() {
        let cases = [
            ("new", "(context)", true),
            ("new", "", false),
            ("", "(context)", false),
            ("renew", "(context)", false),
        ];
        for (word, tail, expected) in cases {
            assert_eq!(
                HelperNewCall::matches(word, tail),
                expected,
                "{word} {tail}"
            );
            let before = format!("value = {word} ");
            let after = format!("  {tail}");
            assert_eq!(
                HelperNewCall::matches(&before, &after),
                expected,
                "{word} {tail}"
            );
        }
    }
}
