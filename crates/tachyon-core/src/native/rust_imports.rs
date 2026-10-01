use crate::Failure;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path;

#[derive(Default)]
pub(super) struct RustImports {
    statements: BTreeSet<String>,
    bindings: BTreeMap<String, String>,
}

impl RustImports {
    pub(super) fn take(&mut self, path: &Path, source: &str) -> Result<String, Failure> {
        let masked = crate::annotation_layout::masked_source(path, source)?;
        let ranges = Self::ranges(&masked);
        let mut body = source.as_bytes().to_vec();
        ranges
            .into_iter()
            .try_for_each(|range| self.consume(path, &masked, &mut body, range))?;
        String::from_utf8(body)
            .map_err(|_| Self::failure(path, "Native Rust imports are not UTF-8."))
    }

    fn consume(
        &mut self,
        path: &Path,
        masked: &str,
        body: &mut [u8],
        range: Range<usize>,
    ) -> Result<(), Failure> {
        let statement = masked[range.clone()].trim();
        let tree = statement
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .trim();
        self.tree(path, tree, "")?;
        Self::erase(body, range);
        Ok(())
    }

    pub(super) fn source(&self) -> String {
        self.statements
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn ranges(masked: &str) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let mut offset = 0;
        let mut depth = 0;
        let mut attributed = false;
        for line in masked.split_inclusive('\n') {
            Self::add_range(masked, (line, offset), (depth, attributed), &mut ranges);
            depth = crate::annotation_layout::brace_depth_after(line, depth);
            attributed = line.trim_start().starts_with("#[");
            offset += line.len();
        }
        ranges
    }

    fn add_range(
        masked: &str,
        positioned: (&str, usize),
        context: (usize, bool),
        ranges: &mut Vec<Range<usize>>,
    ) {
        let (line, offset) = positioned;
        if context != (0, false) || !line.trim_start().starts_with("use ") {
            return;
        }
        let start = offset + line.len() - line.trim_start().len();
        ranges.extend(masked[start..].find(';').map(|end| start..start + end + 1));
    }

    fn erase(body: &mut [u8], range: Range<usize>) {
        body[range].iter_mut().for_each(Self::blank);
    }

    fn blank(byte: &mut u8) {
        if !b"\n\r".contains(byte) {
            *byte = b' ';
        }
    }

    fn tree(&mut self, path: &Path, tree: &str, prefix: &str) -> Result<(), Failure> {
        let tree = tree.trim();
        if let Some(open) = tree.find('{') {
            return self.group(path, tree, (prefix, open));
        }
        self.leaf(path, tree, prefix)
    }

    fn group(&mut self, path: &Path, tree: &str, context: (&str, usize)) -> Result<(), Failure> {
        let (prefix, open) = context;
        let inner = tree[open + 1..].strip_suffix('}').ok_or_else(|| {
            Self::failure(
                path,
                "Native Rust use tree has an unsupported trailing form.",
            )
        })?;
        let prefix = format!("{prefix}{}", tree[..open].trim());
        let entries = Self::entries(path, inner)?;
        entries
            .iter()
            .try_for_each(|entry| self.tree(path, entry, &prefix))
    }

    fn entries<'tree>(path: &Path, tree: &'tree str) -> Result<Vec<&'tree str>, Failure> {
        let mut depth = 0_i32;
        let mut start = 0;
        let mut entries = Vec::new();
        tree.bytes().enumerate().for_each(|(index, byte)| {
            depth += i32::from(byte == b'{') - i32::from(byte == b'}');
            Self::entry(tree, (index, byte, depth), &mut start, &mut entries);
        });
        Self::require_balanced(path, depth)?;
        entries.extend((!tree[start..].trim().is_empty()).then_some(&tree[start..]));
        Ok(entries)
    }

    fn entry<'tree>(
        tree: &'tree str,
        position: (usize, u8, i32),
        start: &mut usize,
        entries: &mut Vec<&'tree str>,
    ) {
        if (position.1, position.2) == (b',', 0) {
            entries.push(&tree[*start..position.0]);
            *start = position.0 + 1;
        }
    }

    fn require_balanced(path: &Path, depth: i32) -> Result<(), Failure> {
        if depth != 0 {
            return Err(Self::failure(path, "Native Rust use tree is unbalanced."));
        }
        Ok(())
    }

    fn leaf(&mut self, path: &Path, tree: &str, prefix: &str) -> Result<(), Failure> {
        let (item, alias) = Self::alias(tree);
        let qualified = Self::qualified(prefix, item)
            .split_whitespace()
            .collect::<String>();
        let binding = alias.unwrap_or_else(|| qualified.rsplit("::").next().unwrap_or(""));
        self.require_binding(path, binding, &qualified)?;
        let suffix = alias.map_or_else(String::new, |alias| format!(" as {alias}"));
        self.statements.insert(format!("use {qualified}{suffix};"));
        Ok(())
    }

    fn alias(tree: &str) -> (&str, Option<&str>) {
        let tree = tree.trim();
        tree.split_once(" as ")
            .map_or((tree, None), |(name, alias)| {
                (name.trim(), Some(alias.trim()))
            })
    }

    fn qualified(prefix: &str, item: &str) -> String {
        if item == "self" {
            return prefix.trim_end_matches("::").to_owned();
        }
        format!("{prefix}{item}")
    }

    fn require_binding(
        &mut self,
        path: &Path,
        binding: &str,
        qualified: &str,
    ) -> Result<(), Failure> {
        let existing = (!["*", "_"].contains(&binding))
            .then(|| {
                self.bindings
                    .insert(binding.to_owned(), qualified.to_owned())
            })
            .flatten();
        Self::require_same(path, existing.as_deref(), qualified)
    }

    fn require_same(path: &Path, existing: Option<&str>, qualified: &str) -> Result<(), Failure> {
        if existing.is_some_and(|existing| existing != qualified) {
            return Err(Self::failure(
                path,
                "Native Rust role imports bind the same name to different types; use distinct explicit aliases in the authored files.",
            ));
        }
        Ok(())
    }

    fn failure(path: &Path, message: &str) -> Failure {
        Failure::one(crate::failure::diagnostic(
            1406,
            message,
            None,
            crate::failure::source_span(&path.to_string_lossy(), 0, 0),
        ))
    }
}
