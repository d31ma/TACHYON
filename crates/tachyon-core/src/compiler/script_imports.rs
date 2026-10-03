use super::script_import::ScriptImport;
use std::ops::Range;

pub(crate) struct ScriptImports;

impl ScriptImports {
    pub(crate) fn read(source: &str) -> Vec<ScriptImport> {
        let mask = crate::lexical::code_mask("js", source);
        let mut imports: Vec<_> = ["import", "from"]
            .into_iter()
            .flat_map(|keyword| Self::matching(source, &mask.code, keyword))
            .collect();
        imports.sort_by_key(|import| import.range.start);
        imports.dedup_by_key(|import| import.range.start);
        imports
    }

    fn matching(source: &str, code: &[bool], keyword: &str) -> Vec<ScriptImport> {
        source
            .match_indices(keyword)
            .filter(|(at, _)| Self::keyword(source, code, *at, keyword))
            .filter_map(|(at, _)| Self::specifier(source, at + keyword.len(), keyword))
            .map(|range| ScriptImport {
                specifier: source[range.clone()].to_owned(),
                range,
            })
            .collect()
    }

    fn keyword(source: &str, code: &[bool], at: usize, keyword: &str) -> bool {
        let before = source[..at].chars().next_back();
        let after = source[at + keyword.len()..].chars().next();
        [
            code.get(at) == Some(&true),
            !before.is_some_and(Self::word),
            before != Some('.'),
            !after.is_some_and(Self::word),
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn word(value: char) -> bool {
        [value.is_ascii_alphanumeric(), "_$".contains(value)].contains(&true)
    }

    fn specifier(source: &str, start: usize, keyword: &str) -> Option<Range<usize>> {
        let rest = Self::argument(source[start..].trim_start(), keyword);
        let quote = rest.chars().next()?;
        "'\"".contains(quote).then_some(())?;
        let offset = source.len() - rest.len() + 1;
        let end = rest[1..].find(quote)?;
        Some(offset..offset + end)
    }

    fn argument<'a>(rest: &'a str, keyword: &str) -> &'a str {
        if [keyword == "import", rest.starts_with('(')]
            .into_iter()
            .all(|holds| holds)
        {
            rest[1..].trim_start()
        } else {
            rest
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScriptImports;

    #[test]
    fn script_imports_capture_static_exports_and_literal_dynamic_imports() {
        let source = "import {\n text\n} from '../shared/utils/TextUtils.js';\nimport './side.js';\nexport { more } from '/shared/more.mjs';\nconst module = import('./lazy.js');";
        let found: Vec<_> = ScriptImports::read(source)
            .into_iter()
            .map(|import| source[import.range].to_owned())
            .collect();
        assert_eq!(
            found,
            [
                "../shared/utils/TextUtils.js",
                "./side.js",
                "/shared/more.mjs",
                "./lazy.js"
            ]
        );
    }

    #[test]
    fn script_imports_ignore_comments_strings_and_member_names() {
        let source = "// import './missing.js';\nconst text = \"from './missing.js'\";\nobject.import('./missing.js');\nconst from = './missing.js';\nimport.meta.url;";
        assert!(ScriptImports::read(source).is_empty());
    }
}
