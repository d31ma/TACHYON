use crate::Failure;
use crate::ascii_name::AsciiName;
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
    const FACTORIES: [&'static str; 7] = [
        "behaviors.construct(",
        "behaviors.construct(typeof(",
        "behaviors.construct<",
        "behaviors.construct::<",
        "behaviors::construct::<",
        "behaviors::construct(",
        "behaviorsconstruct[",
    ];

    pub(crate) fn read(path: &Path, source: &str) -> Result<BTreeSet<String>, Failure> {
        let candidates = super::parser::HelperSources::read(path, source)?;
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
        let results = (extension == "go")
            .then(|| crate::go_source::GoFunctionResults::read(code))
            .unwrap_or_default();
        code.match_indices(&helper.name).any(|(at, _)| {
            [
                !Self::go_result(&results, at),
                Self::call_matches(code, helper, extension, at),
            ] == [true; 2]
        })
    }

    fn go_result(results: &[std::ops::Range<usize>], at: usize) -> bool {
        results.iter().any(|result| result.contains(&at))
    }

    fn call_matches(code: &str, helper: &HelperSource, extension: &str, at: usize) -> bool {
        let after = &code[at + helper.name.len()..];
        let before = &code[..at];
        let constructor = Self::constructor_call(extension, before, after);
        let definition = [extension == "dart", helper.body.contains(&at)] == [true, true];
        let factory = Self::factory_call(extension, before, after);
        let declaration = (helper.declaration..helper.body.start).contains(&at);
        let return_type = [extension == "rs", Self::rust_return_type(before)] == [true, true];
        let rust_declaration =
            [extension == "rs", Self::rust_declaration_context(before)] == [true, true];
        [
            Self::name_boundary(before, after),
            ![declaration, definition, return_type, rust_declaration].contains(&true),
            [constructor, factory].contains(&true),
        ] == [true, true, true]
    }

    fn name_boundary(before: &str, after: &str) -> bool {
        let previous = before
            .as_bytes()
            .last()
            .copied()
            .is_some_and(AsciiName::is_name_byte);
        let next = after
            .as_bytes()
            .first()
            .copied()
            .is_some_and(AsciiName::is_name_byte);
        ![previous, next].contains(&true)
    }

    fn constructor_call(extension: &str, before: &str, after: &str) -> bool {
        let word = before.split_whitespace().next_back().unwrap_or("");
        let check: fn(&str, &str) -> bool = match extension {
            "rs" => Self::rust_call,
            "rb" => Self::ruby_call,
            "go" => Self::go_call,
            "py" | "kt" | "dart" => Self::direct_call,
            _ => crate::helper_new_call::HelperNewCall::matches,
        };
        check(word, after.trim_start())
    }

    fn rust_call(word: &str, tail: &str) -> bool {
        let declaration = ["struct", "impl", "type", "for", "as", "->"].contains(&word);
        [!declaration, Self::rust_invocation(tail)] == [true, true]
    }

    fn rust_invocation(tail: &str) -> bool {
        let associated = Self::rust_associated(tail);
        [associated, tail.starts_with('{')].contains(&true)
    }

    fn rust_associated(tail: &str) -> bool {
        let Some(method) = tail.strip_prefix("::") else {
            return false;
        };
        let length = method
            .bytes()
            .take_while(|byte| AsciiName::is_name_byte(*byte))
            .count();
        [length > 0, method[length..].trim_start().starts_with('(')] == [true, true]
    }

    fn rust_declaration_context(before: &str) -> bool {
        let tail = before.split_whitespace().next_back().unwrap_or("");
        let prefix = before.trim_end().strip_suffix(tail).unwrap_or(before);
        let word = if tail.ends_with("::") {
            prefix.split_whitespace().next_back().unwrap_or("")
        } else {
            tail
        };
        ["struct", "impl", "type", "for", "as"].contains(&word)
    }

    fn rust_return_type(before: &str) -> bool {
        before
            .rsplit_once("->")
            .is_some_and(|(_, tail)| tail.chars().all(Self::type_path_character))
    }

    fn type_path_character(character: char) -> bool {
        [
            character.is_ascii_alphanumeric(),
            character.is_whitespace(),
            ['_', ':', '&', '*', '\''].contains(&character),
        ]
        .contains(&true)
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

    fn named_call(tail: &str, name: &str) -> bool {
        tail.strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with('('))
    }

    fn factory_call(extension: &str, before: &str, after: &str) -> bool {
        let prefix: String = before.chars().rev().take(Self::CALL_PREFIX).collect();
        let prefix: String = prefix
            .chars()
            .rev()
            .filter(|character| !character.is_whitespace())
            .collect();
        let prefix = prefix.to_ascii_lowercase();
        Self::FACTORIES.iter().any(|factory| {
            [
                Self::factory_boundary(&prefix, factory),
                Self::factory_tail(factory, after),
                [*factory != "behaviors::construct(", extension == "php"].contains(&true),
            ] == [true, true, true]
        })
    }

    fn factory_tail(factory: &str, after: &str) -> bool {
        let tail = after.trim_start();
        match factory.as_bytes().last() {
            Some(b'<') => Self::named_call(tail, ">"),
            Some(b'[') => Self::named_call(tail, "]"),
            _ if factory == "behaviors::construct(" => tail
                .strip_prefix("::class")
                .is_some_and(|rest| rest.trim_start().starts_with([',', ')'])),
            _ => tail.starts_with([',', ')']),
        }
    }

    fn factory_boundary(prefix: &str, factory: &str) -> bool {
        prefix.strip_suffix(factory).is_some_and(|before| {
            !before
                .as_bytes()
                .last()
                .copied()
                .is_some_and(AsciiName::is_name_byte)
        })
    }

    fn require_owned_lifecycle(
        path: &Path,
        code: &str,
        selected: &[&HelperSource],
    ) -> Result<(), Failure> {
        selected
            .iter()
            .filter(|helper| !helper.zero_argument)
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
    fn rust_associated_invocations_and_literals_prove_manual_ownership() {
        let declaration =
            "#[Helper]\nstruct ProbeHelper {}\nimpl ProbeHelper { fn answer() -> i32 { 1 } }\n";
        for caller in [
            "fn caller() { let value = ProbeHelper::answer(); }",
            "fn caller() { let value = ProbeHelper::default(); }",
            "fn caller() { let value = ProbeHelper {}; }",
            "fn caller() -> ProbeHelper { ProbeHelper {} }",
        ] {
            assert_eq!(
                names("rs", &format!("{declaration}{caller}")),
                BTreeSet::from(["ProbeHelper".into()]),
                "{caller}"
            );
        }
    }

    #[test]
    fn rust_declarations_return_types_and_method_references_are_not_calls() {
        let declaration = "#[Helper]\nstruct ProbeHelper {}\n";
        for reference in [
            "impl ProbeHelper {}",
            "impl Trait for ProbeHelper {}",
            "impl Trait for crate::ProbeHelper {}",
            "fn caller() -> ProbeHelper { panic!() }",
            "fn caller() -> crate::ProbeHelper { panic!() }",
            "fn caller() -> &ProbeHelper { panic!() }",
            "fn caller() { let method = ProbeHelper::answer; }",
            "type Alias = ProbeHelper;",
        ] {
            assert!(
                names("rs", &format!("{declaration}{reference}")).is_empty(),
                "{reference}"
            );
        }
    }

    #[test]
    fn generic_factory_reference_without_call_is_not_construction() {
        let source = "[Helper]\nclass ProbeHelper {\n [Needs]\n public BehaviorContext behaviors;\n}\nclass Caller { object method = Behaviors.construct<ProbeHelper>; }";
        assert!(names("cs", source).is_empty());
    }

    #[test]
    fn php_class_constant_factory_proves_only_real_exact_calls() {
        let declaration = "<?php\n#[Helper]\nclass ProbeHelper {\n #[Needs]\n public BehaviorContext $behaviors;\n}\n";
        let caller =
            "$value = Behaviors::construct(ProbeHelper::class, ['behaviors' => $context]);";
        assert_eq!(
            names("php", &format!("{declaration}{caller}")),
            BTreeSet::from(["ProbeHelper".into()])
        );
        for reference in [
            "$value = OtherBehaviors::construct(ProbeHelper::class, []);",
            "$value = Behaviors::construct(ProbeHelper::classSuffix, []);",
            "$value = ProbeHelper::class;",
            "$method = Behaviors::construct; $type = ProbeHelper::class;",
            "// Behaviors::construct(ProbeHelper::class, []);",
            "$text = 'Behaviors::construct(ProbeHelper::class, [])';",
        ] {
            assert!(
                names("php", &format!("{declaration}{reference}")).is_empty(),
                "{reference}"
            );
        }
    }

    #[test]
    fn php_factory_keeps_provider_optins_and_owned_hook_rejections() {
        let source = "<?php\n#[Helper]\nclass ProbeHelper {\n #[Needs]\n public BehaviorContext $behaviors;\n}\n$value = Behaviors::construct(ProbeHelper::class, ['behaviors' => $context]);";
        let managed = source.replace("#[Helper]", "#[Helper]\n#[NeedsAll]");
        assert!(names("php", &managed).is_empty());
        let hooked = source.replace(
            " public BehaviorContext $behaviors;",
            " public BehaviorContext $behaviors;\n #[OnStart]\n public function start(): void {}\n",
        );
        let failure = ManualHelpers::read(Path::new("yon.php"), &hooked)
            .err()
            .unwrap_or_else(|| panic!("expected caller-owned lifecycle failure"));
        assert!(failure.to_string().contains("TY2015"));
        assert!(failure.to_string().contains("caller-owned"));
    }

    #[test]
    fn zero_argument_called_helper_keeps_managed_lifecycle() {
        let source = "@Helper\nclass ProbeHelper {\n ProbeHelper() {}\n @OnStart\n void start() {}\n}\nclass Caller { Object value = new ProbeHelper(); }";
        assert_eq!(
            names("java", source),
            BTreeSet::from(["ProbeHelper".into()])
        );
    }

    #[test]
    fn field_needs_called_helper_cannot_claim_managed_lifecycle() {
        let source = "[Helper]\nclass ProbeHelper {\n [Needs]\n public BehaviorContext behaviors;\n [OnStart]\n void Start() {}\n}\nclass Caller { object value = Behaviors.construct(typeof(ProbeHelper), context); }";
        let failure = ManualHelpers::read(Path::new("yon.cs"), source)
            .err()
            .unwrap_or_else(|| panic!("expected lifecycle failure"));
        assert!(failure.to_string().contains("caller-owned"));
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
    fn go_function_results_are_not_construction() {
        let declaration = "// @Helper\ntype ProbeHelper struct { value int }\ntype Caller struct { stored *ProbeHelper }\n";
        for reference in [
            "func (caller *Caller) Read() *ProbeHelper { return caller.stored }",
            "func Read() *ProbeHelper { return nil }",
            "func Read() ProbeHelper { return ProbeHelper{} }",
            "func Read() []ProbeHelper { return nil }",
            "func Read() map[string]*ProbeHelper { return nil }",
            "func Read() chan *ProbeHelper { return nil }",
            "func Read() func() *ProbeHelper { return nil }",
            "func Read() { _ = func() *ProbeHelper { return nil } }",
            "func Read() (*ProbeHelper) { return nil }",
            "func Read() (result *ProbeHelper) { return }",
            "func Read() (*ProbeHelper, error) { return nil, nil }",
            "func Read[T any]() *ProbeHelper { return nil }",
            "func Read() *\nProbeHelper { return nil }",
            "func Read() { _ = func() *\nProbeHelper { return nil } }",
        ] {
            assert!(
                names("go", &format!("{declaration}{reference}")).is_empty(),
                "{reference}"
            );
        }
    }

    #[test]
    fn go_function_results_keep_real_literal_construction() {
        let declaration = "// @Helper\ntype ProbeHelper struct { value int }\ntype Caller struct { stored *ProbeHelper }\n";
        for caller in [
            "func (caller *Caller) Read() *ProbeHelper { return &ProbeHelper{value: 1} }",
            "func Read() ProbeHelper { return ProbeHelper{1} }",
            "func Read() *ProbeHelper { return &ProbeHelper{1} }",
            "func Read() { _ = func() *ProbeHelper { return &ProbeHelper{value: 1} } }",
            "var callback func() *ProbeHelper = &ProbeHelper{value: 1}",
            "func Read() { accept((func() *ProbeHelper)(nil), ProbeHelper{value: 1}) }",
            "func Read() *\nProbeHelper { return &ProbeHelper{value: 1} }",
            "func Read() { _ = BehaviorsConstruct[ProbeHelper](context) }",
            "func Read() ProbeHelper { var _ func() func()\n return ProbeHelper{value: 1} }",
        ] {
            assert_eq!(
                names("go", &format!("{declaration}{caller}")),
                BTreeSet::from(["ProbeHelper".into()]),
                "{caller}"
            );
        }
    }

    #[test]
    fn go_result_getter_keeps_injected_helper_and_literal_still_requires_optin() {
        let path = Path::new("server/controllers/C.go");
        let source = "// @Helper\ntype ProbeHelper struct { value int }\n// @RequestRoute(\"/c\")\n// @Needs(ProbeHelper)\ntype C struct { helper *ProbeHelper }\nfunc (caller *C) Read() *ProbeHelper { return caller.helper }\n// @GetRoute\nfunc (caller *C) Get() any { return nil }\n";
        let parsed = super::super::parser::parse(path, source)
            .unwrap_or_else(|failure| panic!("{failure}"))
            .unwrap_or_else(|| panic!("expected managed source"));
        assert!(
            parsed
                .plan
                .providers
                .iter()
                .any(|provider| provider.name == "ProbeHelper")
        );
        let literal = source.replace("return caller.helper", "return &ProbeHelper{value: 1}");
        let failure = super::super::parser::parse(path, &literal)
            .err()
            .unwrap_or_else(|| panic!("expected caller-owned injection failure"));
        assert!(
            failure
                .to_string()
                .contains("uses explicit caller construction")
        );
        let managed = literal.replace("// @Helper", "// @Helper\n// @Needs()");
        assert!(super::super::parser::parse(path, &managed).is_ok());
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
