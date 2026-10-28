use super::{
    CallArguments, ClassClose, DelegateBase, DelegateForwards, FsharpParameters, MethodHeader,
    SourceLines,
};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct TypedForward<'a> {
    language: &'a str,
    base: &'a DelegateBase<'a>,
    delegate: String,
    access: &'static str,
    owner: &'static str,
}

impl<'a> TypedForward<'a> {
    const OVERLOADED: [&'static str; 6] = ["java", "groovy", "kt", "cs", "hpp", "swift"];
    const BRACED: [&'static str; 7] = ["java", "groovy", "kt", "cs", "dart", "swift", "hpp"];
    const AT_END: [&'static str; 2] = ["go", "rs"];
    const MEMBER_STEP: &'static str = "  ";
    const GO_STEP: &'static str = "\t";
    const RUST_STEP: &'static str = "    ";
    const GO_RECEIVER: &'static str = "delegate";
    const RUST_PUBLIC: &'static str = "pub";
    const POINTER_MARKS: [&'static str; 2] = ["*", "_ptr<"];
    const NOT_DECLARED_BEFORE: [&'static str; 2] = ["=", "return"];
    const HIDDEN_WORDS: [&'static str; 5] =
        ["private", "protected", "fileprivate", "internal", "static"];
    const FSHARP_MEMBER: &'static str = "member ";
    const FSHARP_SELF: &'static str = "this";
    const FSHARP_OWNER: &'static str = "this.";

    pub(super) fn written(
        path: &Path,
        (source, masked): (&str, &str),
        base: &'a DelegateBase<'a>,
        methods: &[(String, usize)],
    ) -> Option<String> {
        let language = path.extension()?.to_str()?;
        let delegate = DelegateForwards::class_name(path);
        let access = Self::access(language, masked, &base.field);
        let owner = Self::owner(language, path, source, &base.field);
        let forward = TypedForward {
            language,
            base,
            delegate,
            access,
            owner,
        };
        let service_masked =
            crate::annotation_layout::masked_source(base.service_path, base.service_source).ok()?;
        let headers = forward.headers(&service_masked, methods);
        let (at, indentation) = forward.placed(source, masked)?;
        let text = forward.text(&indentation, &headers);
        (!headers.is_empty()).then(|| format!("{}{text}{}", &source[..at], &source[at..]))
    }

    fn access(language: &str, masked: &str, field: &str) -> &'static str {
        let pointer = masked
            .lines()
            .filter(|line| SourceLines::words(line).contains(field))
            .filter(|line| !line.contains('('))
            .any(|line| Self::POINTER_MARKS.iter().any(|mark| line.contains(mark)));
        [".", "->"][usize::from([language == "hpp", pointer] == [true, true])]
    }

    fn owner(language: &str, path: &Path, source: &str, field: &str) -> &'static str {
        let parameter = FsharpParameters::needed(path, source)
            .iter()
            .any(|needed| needed.name == field);
        ["", Self::FSHARP_OWNER][usize::from([language == "fs", parameter] == [true, false])]
    }

    fn headers(
        &self,
        service_masked: &str,
        methods: &[(String, usize)],
    ) -> Vec<(String, MethodHeader)> {
        let lines = SourceLines::with_offsets(service_masked);
        let mut seen = BTreeSet::new();
        methods
            .iter()
            .flat_map(|(name, line)| {
                self.starts(&lines, name, *line)
                    .into_iter()
                    .map(move |start| (name.clone(), start))
            })
            .filter_map(|(name, start)| {
                let header = MethodHeader::read(
                    self.language,
                    (self.base.service_source, service_masked),
                    start,
                    &name,
                )?;
                Some((name, header))
            })
            .filter(|(_, header)| self.is_public(header))
            .filter(|(_, header)| seen.insert(header.declaration.clone()))
            .collect()
    }

    fn is_public(&self, header: &MethodHeader) -> bool {
        let private_rust = !header.declaration.starts_with(Self::RUST_PUBLIC);
        [self.language == "rs", private_rust] != [true, true]
    }

    fn starts(&self, lines: &[(usize, &str)], name: &str, line: usize) -> Vec<usize> {
        let Some(&(at, text)) = lines.get(line) else {
            return Vec::new();
        };
        let level = SourceLines::indentation(text);
        let overloaded = Self::OVERLOADED.contains(&self.language);
        let others = lines
            .iter()
            .filter(|_| overloaded)
            .filter(|(other, _)| *other != at)
            .filter(|(_, other)| SourceLines::indentation(other) == level)
            .filter(|(_, other)| Self::may_declare(other, name))
            .map(|(other, other_text)| other + SourceLines::indentation(other_text).len());
        std::iter::once(at + level.len()).chain(others).collect()
    }

    fn may_declare(text: &str, name: &str) -> bool {
        let words = SourceLines::words(text);
        let before = text.split(name).next().unwrap_or_default();
        let assigned = Self::NOT_DECLARED_BEFORE
            .iter()
            .any(|word| before.contains(word));
        let hidden = SourceLines::words(before)
            .iter()
            .any(|word| Self::HIDDEN_WORDS.contains(&word.as_str()));
        [words.contains(name), !assigned, !hidden] == [true, true, true]
    }

    fn placed(&self, source: &str, masked: &str) -> Option<(usize, String)> {
        let braced = Self::BRACED.contains(&self.language);
        let at_end = Self::AT_END.contains(&self.language);
        let placements = [
            braced
                .then(|| ClassClose::member_slot(masked, &self.delegate))
                .flatten(),
            at_end.then(|| (source.len(), String::new())),
            (self.language == "fs")
                .then(|| ClassClose::indented(masked, &self.delegate))
                .flatten(),
        ];
        placements.into_iter().flatten().next()
    }

    fn text(&self, indentation: &str, headers: &[(String, MethodHeader)]) -> String {
        let methods: String = headers
            .iter()
            .map(|(name, header)| self.method(indentation, name, header))
            .collect();
        match self.language {
            "rs" => format!("\nimpl {} {{{methods}}}\n", self.delegate),
            "hpp" => format!("\npublic:{methods}"),
            _ => methods,
        }
    }

    fn method(&self, indentation: &str, name: &str, header: &MethodHeader) -> String {
        let field = &self.base.field;
        let access = self.access;
        let arguments = CallArguments::joined(self.language, &header.parameters);
        let owner = self.owner;
        let call = format!("{owner}{field}{access}{name}({arguments})");
        let declaration = &header.declaration;
        let step = Self::MEMBER_STEP;
        let returned = ["", "return "][usize::from(header.returns_value)];
        match self.language {
            "kt" => format!("\n{indentation}{declaration} = {call}\n"),
            "fs" => format!(
                "\n{indentation}{} = {call}\n",
                Self::fsharp_declaration(declaration)
            ),
            "go" => self.go_method(declaration, &call, header.returns_value),
            "rs" => Self::rust_method(declaration, &call),
            "swift" => format!(
                "\n{indentation}{declaration} {{\n{indentation}{step}return {}{call}\n{indentation}}}\n",
                Self::swift_marks(declaration)
            ),
            _ => format!(
                "\n{indentation}{declaration} {{\n{indentation}{step}{returned}{call};\n{indentation}}}\n"
            ),
        }
    }

    fn fsharp_declaration(declaration: &str) -> String {
        declaration
            .strip_prefix(Self::FSHARP_MEMBER)
            .and_then(|rest| rest.split_once('.'))
            .map_or_else(
                || declaration.to_owned(),
                |(_, named)| format!("{}{}.{named}", Self::FSHARP_MEMBER, Self::FSHARP_SELF),
            )
    }

    fn go_method(&self, declaration: &str, call: &str, returns_value: bool) -> String {
        let receiver_end = declaration.find(')').map_or(0, |close| close + 1);
        let rest = &declaration[receiver_end..];
        let receiver = Self::GO_RECEIVER;
        let returned = ["", "return "][usize::from(returns_value)];
        let step = Self::GO_STEP;
        format!(
            "\nfunc ({receiver} *{}){rest} {{\n{step}{returned}{receiver}.{call}\n}}\n",
            self.delegate
        )
    }

    fn rust_method(declaration: &str, call: &str) -> String {
        let awaited = ["", ".await"][usize::from(declaration.contains("async "))];
        let step = Self::RUST_STEP;
        format!("\n{step}{declaration} {{\n{step}{step}self.{call}{awaited}\n{step}}}\n")
    }

    fn swift_marks(declaration: &str) -> String {
        let throws = [" throws", " rethrows"]
            .iter()
            .any(|mark| declaration.contains(mark));
        let waits = declaration.contains(" async");
        format!(
            "{}{}",
            ["", "try "][usize::from(throws)],
            ["", "await "][usize::from(waits)]
        )
    }
}
