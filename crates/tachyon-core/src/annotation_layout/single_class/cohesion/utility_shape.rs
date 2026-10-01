use super::super::super::{Failure, Input};
use super::super::Tokens;
use super::CohesionCheck;
use super::layout::Layout;
use super::method::Method;
use super::spelling::Spelling;
use super::words::Words;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

mod rust_constants;
use rust_constants::RustConstants;

pub(crate) struct UtilityShape<'r, 'a> {
    path: String,
    class_name: String,
    extension: &'r str,
    layout: Layout,
    tokens: &'r Tokens<'a>,
    words: &'r Words<'a>,
    methods: Vec<Method>,
    members: Vec<(Range<usize>, usize)>,
    markers: Vec<Range<usize>>,
    constants: Vec<Range<usize>>,
}

impl UtilityShape<'_, '_> {
    const CODE: u16 = 1316;
    const STRUCT_LANGUAGES: [&'static str; 2] = ["go", "rs"];
    const SEPARATORS: [&'static str; 3] = [";", ",", "}"];
    const STATEMENT_WORDS: [&'static str; 6] = [
        "end",
        "pass",
        "private",
        "protected",
        "public",
        "private_class_method",
    ];
    const PARAMETER_ENDS: [&'static str; 3] = [",", ":", ")"];

    pub(crate) fn problems(path: &Path, source: &str) -> Result<Vec<Diagnostic>, Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Spelling::of(extension).map_or(Ok(Vec::new()), |spelling| {
            Self::scan(path, source, &spelling)
        })
    }

    fn scan(path: &Path, source: &str, spelling: &Spelling) -> Result<Vec<Diagnostic>, Failure> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input)?;
        tokens.check_complete(&input)?;
        let words = Words::read(&tokens, source);
        let shape = UtilityShape {
            path: path.to_string_lossy().replace('\\', "/"),
            class_name: Self::class_name(path),
            extension: input.extension,
            layout: spelling.layout(),
            tokens: &tokens,
            words: &words,
            methods: CohesionCheck::methods(&words, spelling),
            members: Self::member_levels(&words, spelling),
            markers: Self::markers(&tokens),
            constants: RustConstants::read(&words, input.extension),
        };
        Ok(shape.diagnostics())
    }

    fn class_name(path: &Path) -> String {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn member_levels(words: &Words<'_>, spelling: &Spelling) -> Vec<(Range<usize>, usize)> {
        (0..words.len())
            .filter(|at| spelling.is_container(words.text(*at)) & !words.after_member_access(*at))
            .filter_map(|at| words.find_at_depth(at + 1, words.depth(at), &["{"]))
            .map(|open| (words.inside(open), words.depth(open) + 1))
            .collect()
    }

    fn markers(tokens: &Tokens<'_>) -> Vec<Range<usize>> {
        (0..tokens.items.len())
            .filter_map(|start| tokens.marker_end(start).map(|end| start..end))
            .collect()
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut found: Vec<Diagnostic> = self
            .methods
            .iter()
            .filter(|method| !self.is_static(method))
            .map(|method| self.instance_problem(method))
            .collect();
        let state = self.struct_state().or_else(|| self.member_state());
        found.extend(state.map(|at| self.state_problem(at)));
        found
    }

    fn is_static(&self, method: &Method) -> bool {
        match self.extension {
            "kt" => self.words.has_word(0..self.words.len(), &["object"]),
            "rs" => !self.takes_self(method),
            "go" => true,
            "py" => self.has_static_decorator(method),
            "rb" => self.is_defined_on_self(method),
            _ => self.has_static_word(method),
        }
    }

    fn takes_self(&self, method: &Method) -> bool {
        let open = self
            .words
            .find_at_depth(method.head, self.words.depth(method.head), &["("])
            .unwrap_or(method.head);
        (open + 1..self.words.len())
            .take_while(|at| !Self::PARAMETER_ENDS.contains(&self.words.text(*at)))
            .any(|at| self.words.text(at) == "self")
    }

    fn has_static_decorator(&self, method: &Method) -> bool {
        let previous_end = self
            .methods
            .iter()
            .map(|other| other.body.end)
            .filter(|end| *end <= method.head)
            .max()
            .unwrap_or(0);
        self.words
            .has_word(previous_end..method.head, &["staticmethod"])
    }

    fn is_defined_on_self(&self, method: &Method) -> bool {
        (self.words.text(method.head + 1) == "self") & (self.words.text(method.head + 2) == ".")
    }

    fn has_static_word(&self, method: &Method) -> bool {
        let head = self.words.line_begin(method.head)..method.body.start.max(method.head);
        self.words.has_word(head, &["static"])
    }

    fn struct_state(&self) -> Option<usize> {
        let checked = Self::STRUCT_LANGUAGES.contains(&self.extension);
        (0..self.words.len())
            .filter(|at| checked & (self.words.text(*at) == "struct"))
            .find_map(|at| self.struct_field(at))
    }

    fn struct_field(&self, at: usize) -> Option<usize> {
        let open = self
            .words
            .find_at_depth(at + 1, self.words.depth(at), &["{", "(", ";"])?;
        let pair = format!("{}{}", self.words.text(open), self.words.text(open + 1));
        let empty = ["{}", "()"].contains(&pair.as_str()) | (self.words.text(open) == ";");
        (!empty).then_some(open + 1)
    }

    fn member_state(&self) -> Option<usize> {
        (0..self.words.len()).find(|at| self.rust_static_declaration(*at) | self.is_state(*at))
    }

    fn rust_static_declaration(&self, at: usize) -> bool {
        let name = at + 1 + usize::from(self.words.text(at + 1) == "mut");
        let declaration = [
            self.extension == "rs",
            self.words.text(at) == "static",
            self.words.is_name(name),
            self.words.text(name + 1) == ":",
        ];
        declaration.into_iter().all(|valid| valid)
    }

    fn is_state(&self, at: usize) -> bool {
        let inside = self.inside_class(at);
        let outside_methods = !self
            .methods
            .iter()
            .any(|method| self.span(method).contains(&at));
        let marker = self.in_marker(at);
        let constant = self.constants.iter().any(|range| range.contains(&at));
        let text = self.words.text(at);
        let allowed = Self::SEPARATORS.contains(&text)
            | Self::STATEMENT_WORDS.contains(&text)
            | self.tokens.items.get(at).is_some_and(|token| token.literal);
        let instance_variable = (self.extension == "rb") & (text == "@");
        (inside & outside_methods & !marker & !constant & !allowed) | instance_variable
    }

    fn inside_class(&self, at: usize) -> bool {
        match self.layout {
            Layout::Braced => self
                .members
                .iter()
                .any(|(range, depth)| range.contains(&at) & (self.words.depth(at) == *depth)),
            Layout::Indented | Layout::Ended => {
                self.words.line_start(at) & (self.words.column(at) > 0)
            }
        }
    }

    fn span(&self, method: &Method) -> Range<usize> {
        let closing = usize::from(self.layout == Layout::Ended);
        let start = if self.extension == "rs" {
            RustConstants::method_start(self.words, method.head)
        } else {
            self.words.line_begin(method.head)
        };
        start..method.body.end + closing
    }

    fn in_marker(&self, at: usize) -> bool {
        self.markers.iter().any(|marker| marker.contains(&at))
    }

    fn instance_problem(&self, method: &Method) -> Diagnostic {
        let message = format!(
            "Method `{}` of the utility class `{}` is not static. A utility class holds only static methods and no state.",
            method.name, self.class_name
        );
        let help = format!("{} (ADR 0064)", self.static_hint(&method.name));
        self.problem(method.head, message, help)
    }

    fn static_hint(&self, name: &str) -> String {
        match self.extension {
            "kt" => format!(
                "Declare the utility as `object {}`; its functions are then static.",
                self.class_name
            ),
            "rs" => format!(
                "Remove the `self` parameter of `{name}`, so that the function belongs to the type."
            ),
            "py" => format!("Put `@staticmethod` above `{name}` and remove its `self` parameter."),
            "rb" => format!("Define it as `def self.{name}`."),
            _ => format!("Add `static` to `{name}`."),
        }
    }

    fn state_problem(&self, at: usize) -> Diagnostic {
        let message = format!(
            "The utility class `{}` holds a field or a stored value at `{}`. A utility class holds only static methods and no state.",
            self.class_name,
            self.words.text(at)
        );
        let help = String::from(
            "Return the value from a static method, or keep the state in a service (ADR 0064).",
        );
        self.problem(at, message, help)
    }

    fn problem(&self, at: usize, message: String, help: String) -> Diagnostic {
        let start = self.words.start(at);
        let end = start + self.words.text(at).len().max(1);
        diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(&self.path, start, end),
        )
    }
}
