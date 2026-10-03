use super::css_imports::CssImports;
use super::css_token::CssToken;
use super::css_tokens::CssTokens;
use super::foundation_rules::FoundationRules;
use super::style_file::StyleFile;
use super::style_path::StylePath;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::native::{SiteSettings, StyleSource};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) struct StyleSources<'a> {
    snapshot: &'a Path,
    declarations: &'a BTreeMap<String, StyleSource>,
    active: BTreeSet<String>,
    visited: BTreeSet<String>,
    total_bytes: usize,
}

impl<'a> StyleSources<'a> {
    const MAX_FILES: usize = 128;
    const MAX_TOTAL_BYTES: usize = 8 * 1_048_576;
    const MAX_DEPTH: usize = 32;

    pub(crate) fn validate(snapshot: &'a Path, site: &'a SiteSettings) -> Result<(), Failure> {
        let mut graph = Self {
            snapshot,
            declarations: &site.style_sources,
            active: BTreeSet::new(),
            visited: BTreeSet::new(),
            total_bytes: 0,
        };
        site.styles.iter().try_for_each(|path| graph.visit(path, 0))
    }

    fn visit(&mut self, path: &str, depth: usize) -> Result<(), Failure> {
        Self::require(
            depth <= Self::MAX_DEPTH,
            path,
            "CSS import graph exceeds 32 levels.",
        )?;
        Self::require(
            !self.active.contains(path),
            path,
            "CSS import graph contains a cycle.",
        )?;
        self.visit_pending(path, depth)
    }

    fn visit_pending(&mut self, path: &str, depth: usize) -> Result<(), Failure> {
        if self.visited.contains(path) {
            return Ok(());
        }
        self.visit_new(path, depth)
    }

    fn visit_new(&mut self, path: &str, depth: usize) -> Result<(), Failure> {
        Self::require(
            self.visited.len() + self.active.len() < Self::MAX_FILES,
            path,
            "CSS import graph exceeds 128 files.",
        )?;
        self.active.insert(String::from(path));
        let bytes = StyleFile::read(self.snapshot, path)
            .map_err(|detail| Self::failure(path, 0, &detail))?;
        self.total_bytes = self.total_bytes.saturating_add(bytes.len());
        Self::require(
            self.total_bytes <= Self::MAX_TOTAL_BYTES,
            path,
            "CSS import graph exceeds 8 MiB.",
        )?;
        let source = std::str::from_utf8(&bytes)
            .map_err(|_| Self::failure(path, 0, "Stylesheets must be UTF-8."))?;
        let tokens = CssTokens::read(crate::without_bom(source))
            .map_err(|detail| Self::failure(path, 0, detail))?;
        self.validate_source(path, &bytes, &tokens)?;
        let imports = CssImports::read(&tokens).map_err(|detail| Self::failure(path, 0, detail))?;
        imports
            .iter()
            .try_for_each(|source| self.visit_import(path, source, depth))?;
        self.active.remove(path);
        self.visited.insert(String::from(path));
        Ok(())
    }

    fn validate_source(
        &self,
        path: &str,
        bytes: &[u8],
        tokens: &[CssToken<'_>],
    ) -> Result<(), Failure> {
        match self.declarations.get(path) {
            Some(StyleSource::Vendor(provenance)) => {
                provenance.verify(&format!("client{path}"), bytes)
            }
            _ => FoundationRules::validate(tokens)
                .map_err(|detail| Self::failure(path, detail.position, detail.message)),
        }
    }

    fn visit_import(&mut self, parent: &str, source: &str, depth: usize) -> Result<(), Failure> {
        let path = StylePath::imported(parent, source)
            .map_err(|detail| Self::failure(parent, 0, detail))?;
        self.visit(&path, depth + 1)
    }

    fn require(valid: bool, path: &str, detail: &str) -> Result<(), Failure> {
        if !valid {
            return Err(Self::failure(path, 0, detail));
        }
        Ok(())
    }

    fn failure(path: &str, position: usize, detail: &str) -> Failure {
        let path = format!("client{path}");
        Failure::one(diagnostic(
            1403,
            format!("Cannot publish global stylesheet '{path}': {detail}"),
            Some(String::from(
                "Keep shared foundation CSS to colors, fonts and resets; move application layout into the owning component's tac.css. Declare exact pinned vendors in tac.styleSources.",
            )),
            source_span(&path, position, position),
        ))
    }
}
