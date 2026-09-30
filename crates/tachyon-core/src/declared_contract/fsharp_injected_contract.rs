use super::constructor_parameter::ConstructorParameter;
use std::path::Path;

pub(crate) struct FsharpInjectedContract<'a> {
    contracts: &'a [String],
}

impl<'a> FsharpInjectedContract<'a> {
    const EXTENSION: &'static str = "fs";
    const TYPE_WORD: &'static str = "type ";
    const OPEN: char = '(';
    const COMMA: char = ',';
    const EQUALS: char = '=';
    const SEPARATOR: &'static str = ", ";
    const DEFAULT_INDENT: &'static str = "    ";
    const BINDING: &'static str = "{indent}let {name} = {type}()\n";
    const OPENERS: [char; 3] = ['(', '<', '['];
    const CLOSERS: [char; 3] = [')', '>', ']'];

    pub(crate) fn filled(path: &Path, source: &str, contracts: &'a [String]) -> Option<String> {
        let fsharp = path.extension().and_then(|value| value.to_str()) == Some(Self::EXTENSION);
        fsharp.then_some(())?;
        let injected = Self { contracts };
        let lines: Vec<&str> = source.split_inclusive('\n').collect();
        let written: Vec<Option<String>> = (0..lines.len())
            .map(|at| injected.header(&lines, at))
            .collect();
        written.iter().any(Option::is_some).then(|| {
            lines
                .iter()
                .zip(written)
                .map(|(line, header)| header.unwrap_or_else(|| (*line).to_owned()))
                .collect()
        })
    }

    fn header(&self, lines: &[&str], at: usize) -> Option<String> {
        let line = lines[at];
        line.starts_with(Self::TYPE_WORD).then_some(())?;
        let open = line.find(Self::OPEN)?;
        let equals = line.find(Self::EQUALS)?;
        (open < equals).then_some(())?;
        let close = Self::group_end(line, open)?;
        let parameters = Self::parameters(&line[open + 1..close]);
        let (injected, kept): (Vec<_>, Vec<_>) = parameters.iter().partition(|parameter| {
            self.contracts
                .iter()
                .any(|contract| contract == parameter.type_name)
        });
        (!injected.is_empty()).then_some(())?;
        let kept: Vec<&str> = kept.iter().map(|parameter| parameter.text).collect();
        let indent = Self::body_indent(&lines[at + 1..]);
        let bindings: String = injected
            .iter()
            .map(|parameter| {
                Self::BINDING
                    .replace("{indent}", indent)
                    .replace("{name}", parameter.name)
                    .replace("{type}", parameter.type_name)
            })
            .collect();
        let header = format!(
            "{}{}{}",
            &line[..=open],
            kept.join(Self::SEPARATOR),
            &line[close..]
        );
        Some(format!("{header}{bindings}"))
    }

    fn parameters(text: &str) -> Vec<ConstructorParameter<'_>> {
        Self::top_level_parts(text)
            .into_iter()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ConstructorParameter::read)
            .collect()
    }

    fn top_level_parts(text: &str) -> Vec<&str> {
        let commas = Self::top_level_commas(text);
        let starts = std::iter::once(0).chain(commas.iter().map(|at| at + 1));
        let ends = commas.iter().copied().chain(std::iter::once(text.len()));
        starts
            .zip(ends)
            .map(|(start, end)| &text[start..end])
            .collect()
    }

    fn top_level_commas(text: &str) -> Vec<usize> {
        text.char_indices()
            .scan(0_i32, |depth, (at, character)| {
                *depth += i32::from(Self::OPENERS.contains(&character))
                    - i32::from(Self::CLOSERS.contains(&character));
                Some((*depth, at, character))
            })
            .filter(|(depth, _, character)| {
                [*depth == 0, *character == Self::COMMA] == [true, true]
            })
            .map(|(_, at, _)| at)
            .collect()
    }

    fn group_end(line: &str, open: usize) -> Option<usize> {
        line.char_indices()
            .skip_while(|(at, _)| *at < open)
            .scan(0_i32, |depth, (at, character)| {
                *depth += i32::from(Self::OPENERS.contains(&character))
                    - i32::from(Self::CLOSERS.contains(&character));
                Some((*depth, at))
            })
            .find(|(depth, _)| *depth == 0)
            .map(|(_, at)| at)
    }

    fn body_indent<'l>(following: &[&'l str]) -> &'l str {
        following
            .iter()
            .find(|line| !line.trim().is_empty())
            .map_or(Self::DEFAULT_INDENT, |line| {
                let indent = &line[..line.len() - line.trim_start().len()];
                [indent, Self::DEFAULT_INDENT][usize::from(indent.is_empty())]
            })
    }
}
