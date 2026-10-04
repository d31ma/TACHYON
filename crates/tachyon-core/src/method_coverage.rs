mod arrow_body;
mod brace_body;
mod call_hooks;
mod class_coverage;
mod coverage_evidence;
mod coverage_report;
mod coverage_text;
mod covered_method;
mod csharp_method_lines;
mod fsharp_call_body;
mod fsharp_members;
mod hook_statement;
mod kotlin_body;
mod lua_body;
mod method_body;
mod method_header;
mod method_state;
mod public_method;
mod python_body;
mod recorded_calls;
mod role_class;
mod role_files;
mod ruby_body;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod shared_utilities_tests;
mod source_edit;
mod swift_body;
mod uncovered;
mod unit_vocabularies;

pub(crate) use call_hooks::CallHooks;
pub use class_coverage::ClassCoverage;
pub use coverage_report::CoverageReport;
pub use coverage_text::CoverageText;
pub use covered_method::CoveredMethod;
pub use method_state::MethodState;
pub use recorded_calls::RecordedCalls;
pub use uncovered::Uncovered;

use arrow_body::ArrowBody;
use brace_body::BraceBody;
use coverage_evidence::CoverageEvidence;
use csharp_method_lines::CsharpMethodLines;
use fsharp_call_body::FsharpCallBody;
use fsharp_members::FsharpMembers;
use hook_statement::HookStatement;
use kotlin_body::KotlinBody;
use lua_body::LuaBody;
use method_body::MethodBody;
use method_header::MethodHeader;
use public_method::PublicMethod;
use python_body::PythonBody;
use role_class::RoleClass;
use role_files::RoleFiles;
use ruby_body::RubyBody;
use source_edit::SourceEdit;
use std::collections::BTreeSet;
use swift_body::SwiftBody;
use unit_vocabularies::UnitVocabularies;

pub(crate) struct PublicMethods;

impl PublicMethods {
    const ARROW_LANGUAGES: [&'static str; 2] = ["dart", "cs"];
    const SIGNATURE_LANGUAGES: [&'static str; 2] = ["kt", "swift"];
    const RESERVED: &[&str] = &[
        "if",
        "for",
        "while",
        "switch",
        "catch",
        "return",
        "constructor",
        "init",
        "new",
        "super",
        "this",
        "class",
        "struct",
        "get",
        "set",
        "match",
        "do",
        "else",
        "try",
        "with",
        "print",
    ];

    fn find(language: &str, masked: &str) -> Vec<PublicMethod> {
        if language == "rb" {
            return ruby_methods(masked);
        }
        if language == crate::lua_source::LuaSource::EXTENSION {
            return LuaBody::methods(masked);
        }
        let mut found = Vec::new();
        let mut seen = BTreeSet::new();
        let prepared = CsharpMethodLines::prepare(language, masked);
        for (line_index, line) in prepared.lines().enumerate() {
            let Some(name) = Self::method_on_line(language, line) else {
                continue;
            };
            if seen.insert(name.clone()) {
                found.push(PublicMethod::new(name, line_index));
            }
        }
        found
    }

    fn bodyless_signature(line: &str) -> bool {
        let balanced = line.matches('(').count() == line.matches(')').count();
        let after_parameters = line.rsplit_once(')').map_or("{", |(_, rest)| rest);
        let has_no_body = [balanced, !after_parameters.contains(['{', '='])] == [true; 2];
        [has_no_body, line.trim_end().ends_with("{}")].contains(&true)
    }

    fn method_on_line(language: &str, line: &str) -> Option<String> {
        if language == "go" {
            return go_method_name(line);
        }
        let trimmed = line.trim();
        let is_indented = line.len() != trimmed.len();
        if trimmed.is_empty() || !is_indented {
            return None;
        }
        if private(language, trimmed) || declared(language, trimmed) {
            return None;
        }
        let name = method_name(language, trimmed)?;
        if Self::RESERVED.contains(&name.as_str()) || name.starts_with('_') || name.starts_with('#')
        {
            return None;
        }
        Some(name)
    }
}

fn private(language: &str, line: &str) -> bool {
    let hidden: &[&str] = match language {
        "cs" => return CsharpMethodLines::hidden(line),
        "php" | "java" | "swift" => &["private ", "protected ", "fileprivate ", "static "],
        "kt" => &["private ", "protected ", "internal "],
        "ts" => &["private ", "protected ", "static "],
        "js" | "dart" => &["static "],
        _ => &[],
    };
    hidden.iter().any(|marker| line.starts_with(marker))
}

fn declared(language: &str, line: &str) -> bool {
    if language == "py" {
        return line.ends_with(": ...");
    }
    if PublicMethods::SIGNATURE_LANGUAGES.contains(&language) {
        return PublicMethods::bodyless_signature(line);
    }
    let is_bodyless_statement = line.ends_with(';') && !line.contains('{') && !line.contains("=>");
    let has_empty_body = line.trim_end().ends_with("{}");
    is_bodyless_statement || has_empty_body
}

fn method_name(language: &str, line: &str) -> Option<String> {
    match language {
        "py" => keyword_name(line, "def "),
        "php" => keyword_name(line, "function "),
        "kt" => keyword_name(line, "fun "),
        "swift" => keyword_name(line, "func "),
        "rs" => rust_method_name(line),
        "js" | "ts" | "java" | "cs" | "dart" => braced_method_name(language, line),
        _ => None,
    }
}

fn go_method_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("func")?.trim_start();
    if !rest.starts_with('(') || line.trim_end().ends_with("{}") {
        return None;
    }
    let close = closing_parenthesis(rest, 0)?;
    let name = rest[close + 1..]
        .trim_start()
        .split(['(', '[', ' '])
        .next()?;
    if !name.starts_with(|character: char| character.is_ascii_uppercase()) {
        return None;
    }
    identifier(name)
}

fn rust_method_name(line: &str) -> Option<String> {
    let name = keyword_name(line, "fn ")?;
    let (_, parameters) = line.split_once('(')?;
    if !parameters.trim_start().starts_with('&') {
        return None;
    }
    Some(name)
}

fn braced_method_name(language: &str, line: &str) -> Option<String> {
    let open = line.find('(')?;
    let close = closing_parenthesis(line, open)?;
    let rest = line[close + 1..].trim_start();
    let rest = without_async_modifier(rest).trim_start();
    let has_typescript_return_type =
        language == "ts" && rest.starts_with(':') && line.trim_end().ends_with('{');
    let is_arrow = PublicMethods::ARROW_LANGUAGES.contains(&language) && rest.starts_with("=>");
    let opens_body = rest.starts_with('{') || has_typescript_return_type || is_arrow;
    if !opens_body {
        return None;
    }
    let before_parameters = line[..open].trim_end();
    identifier(before_parameters.rsplit([' ', '\t', '*']).next()?)
}

fn without_async_modifier(text: &str) -> &str {
    for modifier in ["async*", "sync*", "async"] {
        if let Some(rest) = text.strip_prefix(modifier) {
            return rest;
        }
    }
    text
}

fn closing_parenthesis(line: &str, open: usize) -> Option<usize> {
    MethodHeader::closing(line, open).map(|end| end - 1)
}

fn keyword_name(line: &str, keyword: &str) -> Option<String> {
    let rest = line.split_once(keyword)?.1;
    identifier(rest.split(['(', '<', ':', ' ']).next()?)
}

fn identifier(name: &str) -> Option<String> {
    let name = name.trim();
    let mut characters = name.chars();
    let first = characters.next()?;
    if !first.is_ascii_alphabetic() && first != '_' && first != '$' {
        return None;
    }
    if !characters.all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '$') {
        return None;
    }
    Some(name.to_owned())
}

fn ruby_methods(masked: &str) -> Vec<PublicMethod> {
    let names = crate::ruby_source::RubyMembers::read(masked).methods;
    names
        .into_iter()
        .filter_map(|name| {
            ruby_definition_line(masked, &name).map(|line| PublicMethod::new(name, line))
        })
        .collect()
}

fn ruby_definition_line(masked: &str, name: &str) -> Option<usize> {
    masked
        .lines()
        .position(|line| ruby_definition_name(line).as_deref() == Some(name))
}

fn ruby_definition_name(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("def ")?;
    let name = rest.split(['(', ' ', ';']).next()?;
    Some(name.to_owned())
}

#[cfg(test)]
mod csharp_layout_tests;
#[cfg(test)]
mod tests;
