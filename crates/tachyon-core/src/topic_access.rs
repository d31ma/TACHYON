use crate::behavior::literal::class_reference;
use crate::yon::route_access::RouteAccess;
use std::ops::Range;

pub(crate) struct TopicAccess;

impl TopicAccess {
    const PUBLIC: &'static str = "public";
    const SIGNED_IN: &'static str = "signedin";
    const ALLOW: &'static str = "allow";
    const NAMES: [&'static str; 3] = [Self::PUBLIC, Self::SIGNED_IN, Self::ALLOW];
    const DECORATION_LEADERS: [&'static str; 7] = ["@", "#", "[", "//", "/*", "--", "*"];
    const COMMENT_LEADERS: [&'static str; 6] = ["/**", "///", "//", "--", "# ", "*"];
    const BLOCK_OPENING: &'static str = "/**";
    const BLOCK_CLOSING: &'static str = "*/";
    const FSHARP: &'static str = "fs";
    const LIST_EDGES: [char; 4] = ['{', '}', '[', ']'];
    const ONE_RULE: &'static str = "A topic has one access rule. Keep one of @Public, @SignedIn or @Allow(Role) next to @Topic: @Public means anyone may read the topic stream, @SignedIn means any signed-in user may read it, and @Allow(Role) means only users with one of these roles may read it";
    const NO_ARGUMENTS: &'static str = "@Public and @SignedIn take no arguments. To name the roles that may read the topic, write @Allow(Role), for example @Allow(Admin)";
    const ROLE_ADVICE: &'static str = "@Allow takes one or more role types from server/roles/, for example @Allow(Admin) or @Allow(Admin, Editor). Java writes Admin.class, Kotlin and PHP Admin::class, C# typeof(Admin) and F# typeof<Admin>";

    pub(crate) fn spans(source: &str, marker: &Range<usize>) -> Vec<Range<usize>> {
        Self::block(source, marker)
            .into_iter()
            .filter_map(|line| Self::marker_span(source, &line))
            .collect()
    }

    pub(crate) fn rule(
        extension: &str,
        source: &str,
        marker: &Range<usize>,
    ) -> Result<RouteAccess, (usize, &'static str)> {
        let spans = Self::spans(source, marker);
        Self::refuse_second(&spans)?;
        spans.first().map_or(Ok(RouteAccess::Missing), |span| {
            Self::read_at(extension, source, span)
        })
    }

    pub(crate) fn uncommented(line: &str) -> &str {
        let text = line.trim();
        let text = text
            .strip_suffix(Self::BLOCK_CLOSING)
            .unwrap_or(text)
            .trim_end();
        Self::COMMENT_LEADERS
            .iter()
            .find_map(|leader| text.strip_prefix(leader))
            .map_or(text, str::trim_start)
    }

    fn refuse_second(spans: &[Range<usize>]) -> Result<(), (usize, &'static str)> {
        spans
            .get(1)
            .map_or(Ok(()), |second| Err((second.start, Self::ONE_RULE)))
    }

    fn read_at(
        extension: &str,
        source: &str,
        span: &Range<usize>,
    ) -> Result<RouteAccess, (usize, &'static str)> {
        let (name, arguments) = Self::written(&source[span.clone()]).unwrap_or_default();
        Self::access(extension, &name, arguments).map_err(|message| (span.start, message))
    }

    fn access(extension: &str, name: &str, arguments: &str) -> Result<RouteAccess, &'static str> {
        match (name, arguments.is_empty()) {
            (Self::PUBLIC, true) => Ok(RouteAccess::Public),
            (Self::SIGNED_IN, true) => Ok(RouteAccess::SignedIn),
            (Self::ALLOW, false) => Self::roles(extension, arguments).map(RouteAccess::Allow),
            (Self::ALLOW, true) => Err(Self::ROLE_ADVICE),
            _ => Err(Self::NO_ARGUMENTS),
        }
    }

    fn roles(extension: &str, arguments: &str) -> Result<Vec<String>, &'static str> {
        arguments
            .trim()
            .trim_matches(Self::LIST_EDGES)
            .split(',')
            .map(|text| Self::role(extension, text.trim()))
            .collect()
    }

    fn role(extension: &str, text: &str) -> Result<String, &'static str> {
        let text = Self::without_fsharp_typeof(extension, text);
        class_reference(extension, text)
            .map(str::to_owned)
            .ok_or(Self::ROLE_ADVICE)
    }

    fn without_fsharp_typeof<'a>(extension: &str, text: &'a str) -> &'a str {
        Self::between(text, "typeof<", ">")
            .map(str::trim)
            .filter(|_| extension == Self::FSHARP)
            .unwrap_or(text)
    }

    fn block(source: &str, marker: &Range<usize>) -> Vec<Range<usize>> {
        let lines = Self::lines(source);
        let at = lines
            .iter()
            .position(|line| line.contains(&marker.start))
            .unwrap_or(lines.len());
        let above = Self::run(
            source,
            lines.get(..at).unwrap_or_default().iter().rev(),
            Self::opens_block,
        );
        let below = Self::run(
            source,
            lines.get(at + 1..).unwrap_or_default().iter(),
            Self::closes_block,
        );
        let ended = lines
            .get(at)
            .is_some_and(|line| Self::closes_block(source, line));
        let below = below.into_iter().filter(|_| !ended);
        lines
            .get(at)
            .cloned()
            .into_iter()
            .chain(above)
            .chain(below)
            .collect()
    }

    fn run<'a>(
        source: &str,
        lines: impl Iterator<Item = &'a Range<usize>>,
        ends: fn(&str, &Range<usize>) -> bool,
    ) -> Vec<Range<usize>> {
        let decorated: Vec<&Range<usize>> = lines
            .take_while(|line| Self::is_decoration(source, line))
            .collect();
        let last = decorated
            .iter()
            .position(|line| ends(source, line))
            .map_or(decorated.len(), |at| at + 1);
        decorated.into_iter().take(last).cloned().collect()
    }

    fn lines(source: &str) -> Vec<Range<usize>> {
        let mut lines = Vec::new();
        let mut offset = 0;
        for line in source.split_inclusive('\n') {
            let text = line.trim_end_matches(['\r', '\n']);
            lines.push(offset..offset + text.len());
            offset += line.len();
        }
        lines
    }

    fn is_decoration(source: &str, line: &Range<usize>) -> bool {
        let text = source[line.clone()].trim();
        Self::DECORATION_LEADERS
            .iter()
            .any(|leader| text.starts_with(leader))
    }

    fn opens_block(source: &str, line: &Range<usize>) -> bool {
        let text = source[line.clone()].trim();
        text.starts_with(Self::BLOCK_OPENING) && !text.ends_with(Self::BLOCK_CLOSING)
    }

    fn closes_block(source: &str, line: &Range<usize>) -> bool {
        let text = source[line.clone()].trim();
        text.ends_with(Self::BLOCK_CLOSING) && !text.starts_with(Self::BLOCK_OPENING)
    }

    fn marker_span(source: &str, line: &Range<usize>) -> Option<Range<usize>> {
        let whole = &source[line.clone()];
        let text = Self::uncommented(whole);
        Self::written(text)?;
        let start = line.start + whole.find(text)?;
        Some(start..start + text.len())
    }

    fn written(text: &str) -> Option<(String, &str)> {
        let (inner, lints) = Self::unwrapped(text)?;
        let length = inner
            .bytes()
            .take_while(|byte| Self::name_byte(*byte))
            .count();
        let written = &inner[..length];
        let name = written.replace('_', "").to_ascii_lowercase();
        Self::known(&name, Self::is_lint(lints, written))?;
        let arguments = Self::arguments(inner[length..].trim())?;
        Some((name, arguments))
    }

    fn name_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    fn is_lint(lints: bool, written: &str) -> bool {
        lints && written.starts_with(|character: char| character.is_ascii_lowercase())
    }

    fn known(name: &str, lint: bool) -> Option<()> {
        let known = Self::NAMES.contains(&name);
        ([known, lint] == [true, false]).then_some(())
    }

    fn arguments(rest: &str) -> Option<&str> {
        rest.is_empty()
            .then_some(rest)
            .or_else(|| Self::between(rest, "(", ")"))
    }

    fn unwrapped(text: &str) -> Option<(&str, bool)> {
        let hashed = Self::between(text, "#[", "]").map(|inner| (inner, true));
        let plain = Self::between(text, "[<", ">]")
            .or_else(|| Self::between(text, "[", "]"))
            .or_else(|| text.strip_prefix('@'))
            .map(|inner| (inner, false));
        hashed.or(plain)
    }

    fn between<'a>(text: &'a str, opening: &str, closing: &str) -> Option<&'a str> {
        text.strip_prefix(opening)
            .and_then(|inner| inner.strip_suffix(closing))
    }
}
