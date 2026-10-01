use crate::Failure;
use std::collections::BTreeSet;
use std::path::Path;

mod helper_source;
pub(super) use helper_source::HelperSource;

pub(crate) struct ManualHelpers;

impl ManualHelpers {
    const MAX_HELPERS: usize = 128;
    const CALL_PREFIX: usize = 256;
    const HOOKS: [&'static str; 3] = ["onstart", "onstop", "onevent"];
    const MARKER_PREFIXES: [&'static str; 5] = ["// @", "#[", "[<", "[", "@"];
    const FACTORIES: [&'static str; 6] = [
        "behaviors.construct(",
        "behaviors.construct(typeof(",
        "behaviors.construct<",
        "behaviors.construct::<",
        "behaviors::construct::<",
        "behaviorsconstruct[",
    ];

    pub(crate) fn read(path: &Path, source: &str) -> Result<BTreeSet<String>, Failure> {
        let candidates = super::parser::helper_sources(path, source)?;
        Self::limit(path, &candidates)?;
        let code = crate::annotation_layout::masked_source(path, source)?;
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let selected = Self::selected(&code, &candidates, extension);
        Self::require_owned_lifecycle(path, &code, &selected)?;
        Ok(selected
            .into_iter()
            .map(|helper| helper.name.clone())
            .collect())
    }

    fn limit(path: &Path, candidates: &[HelperSource]) -> Result<(), Failure> {
        if candidates.len() > Self::MAX_HELPERS {
            return Err(Self::failure(
                path,
                0,
                "Yon permits at most 128 authored Helpers",
            ));
        }
        Ok(())
    }

    fn selected<'a>(
        code: &str,
        candidates: &'a [HelperSource],
        extension: &str,
    ) -> Vec<&'a HelperSource> {
        candidates
            .iter()
            .filter(|helper| Self::explicit_caller(code, helper, extension))
            .collect()
    }

    fn explicit_caller(code: &str, helper: &HelperSource, extension: &str) -> bool {
        if helper.managed {
            return false;
        }
        Self::caller(code, helper, extension)
    }

    fn caller(code: &str, helper: &HelperSource, extension: &str) -> bool {
        code.match_indices(&helper.name)
            .any(|(at, _)| Self::call_matches(code, helper, extension, at))
    }

    fn call_matches(code: &str, helper: &HelperSource, extension: &str, at: usize) -> bool {
        let after = &code[at + helper.name.len()..];
        let before = &code[..at];
        let constructor = Self::constructor_call(extension, before, after);
        let definition = [extension == "dart", helper.body.contains(&at)] == [true, true];
        let factory = Self::factory_call(before);
        let declaration = (helper.declaration..helper.body.start).contains(&at);
        [
            Self::name_boundary(before, after),
            ![declaration, definition].contains(&true),
            [constructor, factory].contains(&true),
        ] == [true, true, true]
    }

    fn name_boundary(before: &str, after: &str) -> bool {
        let previous = before
            .as_bytes()
            .last()
            .copied()
            .is_some_and(Self::name_byte);
        let next = after
            .as_bytes()
            .first()
            .copied()
            .is_some_and(Self::name_byte);
        ![previous, next].contains(&true)
    }

    fn name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn constructor_call(extension: &str, before: &str, after: &str) -> bool {
        let word = before.split_whitespace().next_back().unwrap_or("");
        let check: fn(&str, &str) -> bool = match extension {
            "rs" => Self::rust_call,
            "rb" => Self::ruby_call,
            "go" => Self::go_call,
            "py" | "kt" | "dart" => Self::direct_call,
            _ => Self::new_call,
        };
        check(word, after.trim_start())
    }

    fn rust_call(_word: &str, tail: &str) -> bool {
        Self::named_call(tail, "::new")
    }

    fn ruby_call(_word: &str, tail: &str) -> bool {
        Self::named_call(tail, ".new")
    }

    fn go_call(_word: &str, tail: &str) -> bool {
        tail.strip_prefix('{')
            .is_some_and(|fields| !fields.trim_start().starts_with('}'))
    }

    fn direct_call(word: &str, tail: &str) -> bool {
        [
            !["class", "def", "fun"].contains(&word),
            tail.starts_with('('),
        ] == [true, true]
    }

    fn new_call(word: &str, tail: &str) -> bool {
        [word == "new", tail.starts_with('(')] == [true, true]
    }

    fn named_call(tail: &str, name: &str) -> bool {
        tail.strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with('('))
    }

    fn factory_call(before: &str) -> bool {
        let prefix: String = before.chars().rev().take(Self::CALL_PREFIX).collect();
        let prefix: String = prefix
            .chars()
            .rev()
            .filter(|character| !character.is_whitespace())
            .collect();
        let prefix = prefix.to_ascii_lowercase();
        Self::FACTORIES
            .iter()
            .any(|factory| Self::factory_boundary(&prefix, factory))
    }

    fn factory_boundary(prefix: &str, factory: &str) -> bool {
        prefix.strip_suffix(factory).is_some_and(|before| {
            !before
                .as_bytes()
                .last()
                .copied()
                .is_some_and(Self::name_byte)
        })
    }

    fn require_owned_lifecycle(
        path: &Path,
        code: &str,
        selected: &[&HelperSource],
    ) -> Result<(), Failure> {
        selected
            .iter()
            .try_for_each(|helper| Self::require_helper_lifecycle(path, code, helper))
    }

    fn require_helper_lifecycle(
        path: &Path,
        code: &str,
        helper: &HelperSource,
    ) -> Result<(), Failure> {
        let hook = std::iter::once(&helper.body)
            .chain(&helper.implementations)
            .find_map(|body| {
                Self::hook_offset(&code[body.clone()]).map(|offset| body.start + offset)
            });
        hook.map_or(Ok(()), |at| Err(Self::failure(path, at,
            &format!("Helper {} uses caller-owned construction; lifecycle and event hooks require a declared Needs or NeedsAll provider", helper.name))))
    }

    fn hook_offset(code: &str) -> Option<usize> {
        let mut offset = 0;
        code.split_inclusive('\n').find_map(|line| {
            let at = offset;
            offset += line.len();
            Self::hook_line(line).map(|column| at + column)
        })
    }

    fn hook_line(line: &str) -> Option<usize> {
        let text = line.trim_start();
        let after = Self::MARKER_PREFIXES
            .iter()
            .find_map(|prefix| text.strip_prefix(prefix))?;
        let name = after.split(Self::marker_end).next()?;
        let name = name
            .rsplit([':', '.', '\\'])
            .next()?
            .replace('_', "")
            .to_ascii_lowercase();
        Self::HOOKS
            .contains(&name.as_str())
            .then_some(line.len() - text.len())
    }

    fn marker_end(character: char) -> bool {
        ![
            character.is_ascii_alphanumeric(),
            ['_', ':', '.', '\\'].contains(&character),
        ]
        .contains(&true)
    }

    fn failure(path: &Path, at: usize, message: &str) -> Failure {
        Failure::one(crate::failure::diagnostic(2015, message,
            Some("Keep explicit native construction and ownership, or declare source-local provider dependencies with Needs or NeedsAll before using framework DI and lifecycle.".to_owned()),
            crate::failure::source_span(&path.to_string_lossy(), at, at + 1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(extension: &str, source: &str) -> BTreeSet<String> {
        ManualHelpers::read(Path::new(&format!("yon.{extension}")), source)
            .unwrap_or_else(|failure| panic!("{failure}"))
    }

    #[test]
    fn captures_real_native_constructor_calls() {
        let sources = [
            (
                "java",
                "@Helper\nclass ProbeHelper { final Context context; ProbeHelper(Context context) { this.context = context; } }\nclass User { Object value = new ProbeHelper(context); }",
            ),
            (
                "kt",
                "@Helper\nclass ProbeHelper(val context: Context) {}\nval value = ProbeHelper(context)",
            ),
            (
                "py",
                "@helper\nclass ProbeHelper:\n    def __init__(self, context):\n        self.context = context\nvalue = ProbeHelper(context)\n",
            ),
            (
                "rb",
                "#[Helper]\nclass ProbeHelper\n  def initialize(context)\n    @context = context\n  end\nend\nvalue = ProbeHelper.new(context)\n",
            ),
            (
                "rs",
                "#[Helper]\nstruct ProbeHelper { context: Context }\nimpl ProbeHelper { fn new(context: Context) -> Self { Self { context } } }\nfn use_helper() { let value = ProbeHelper::new(context); }",
            ),
        ];
        for (extension, source) in sources {
            assert_eq!(
                names(extension, source),
                BTreeSet::from(["ProbeHelper".into()]),
                "{extension}"
            );
        }
    }

    #[test]
    fn go_requires_nonempty_caller_owned_literal() {
        let source = "// @Helper\ntype ProbeHelper struct { context *BehaviorContext }\nfunc caller() { value := &ProbeHelper{context: context} }";
        assert_eq!(names("go", source), BTreeSet::from(["ProbeHelper".into()]));
        assert!(names("go", &source.replace("context: context", "")).is_empty());
    }

    #[test]
    fn class_provider_optins_keep_managed_ownership() {
        for marker in ["@Needs()", "@NeedsAll()", "@Lifetime(\"Transient\")"] {
            let source = format!(
                "@Helper\n{marker}\nclass ProbeHelper {{ ProbeHelper(Context context) {{}} }}\nclass User {{ Object value = new ProbeHelper(context); }}"
            );
            assert!(names("java", &source).is_empty(), "{marker}");
        }
    }

    #[test]
    fn strings_comments_and_other_roles_do_not_prove_manual_helpers() {
        let source = "@Helper\nclass ProbeHelper { ProbeHelper(Context context) {} }\n@Service\nclass OtherService { OtherService(Context context) {} }\nclass User { String text = \"new ProbeHelper(context)\"; Object other = new OtherService(context); /* new ProbeHelper(context) */ }";
        assert!(names("java", source).is_empty());
    }

    #[test]
    fn factory_names_must_be_exact() {
        let source = "[Helper]\nclass ProbeHelper {\n [Needs]\n public BehaviorContext behaviors;\n}\nclass Caller { object value = Behaviors.construct(typeof(ProbeHelper), context); }";
        assert_eq!(names("cs", source), BTreeSet::from(["ProbeHelper".into()]));
        assert!(
            names(
                "cs",
                &source.replace("Behaviors.construct", "OtherBehaviors.construct")
            )
            .is_empty()
        );
    }

    #[test]
    fn manually_owned_hooks_have_a_located_failure() {
        let source = "@Helper\nclass ProbeHelper {\n @OnStart\n void start() {}\n ProbeHelper(Context context) {}\n}\nclass Caller { Object value = new ProbeHelper(context); }";
        let failure = ManualHelpers::read(Path::new("yon.java"), source)
            .err()
            .unwrap_or_else(|| panic!("expected lifecycle failure"));
        assert!(failure.to_string().contains("TY2015"));
        assert!(failure.to_string().contains("caller-owned"));
    }

    #[test]
    fn field_needs_does_not_opt_into_class_provider_ownership() {
        let source = "[Helper]\nclass ProbeHelper {\n [Needs]\n public BehaviorContext behaviors;\n}\nclass Caller { object value = Behaviors.construct<ProbeHelper>(context); }";
        assert_eq!(names("cs", source), BTreeSet::from(["ProbeHelper".into()]));
    }
}
