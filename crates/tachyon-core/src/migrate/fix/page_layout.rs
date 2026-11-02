use super::super::page_component::PageComponent;
use super::super::page_markup::PageMarkup;
use super::super::{portable_path, view_sources};
use super::manifest::ManifestMigration;
use super::page_sources::PageSources;
use super::renames::FileMove;
use super::{FileChange, ManualStep};
use crate::compiler::{PageUpgrade, SplitStyles};
use crate::sorted_json::SortedJson;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct PageLayout {
    pub(super) changes: Vec<FileChange>,
    pub(super) moves: Vec<FileMove>,
    pub(super) manual_steps: Vec<ManualStep>,
    root: PathBuf,
    single: bool,
    styles: Vec<String>,
    scripts: Vec<String>,
}

impl PageLayout {
    pub(super) fn plan(root: &Path, planned: &[String]) -> Self {
        let views = view_sources(&root.join(PageComponent::PAGES));
        let names = PageMarkup::component_names(root);
        let mut layout = Self {
            root: root.to_path_buf(),
            single: views.len() == 1,
            ..Self::default()
        };
        views
            .iter()
            .map(|view| portable_path(view.strip_prefix(root).unwrap_or(view)))
            .for_each(|view| layout.add_page(planned, &names, &view));
        layout.add_manifest(planned);
        layout
    }

    pub(super) fn is_allowed_move(from: &str, to: &str) -> bool {
        let folder = from.rsplit_once('/').map_or("", |(folder, _)| folder);
        let component = PageComponent::of(folder);
        let targets = component.map(|component| Self::targets(&component, from));
        targets.is_some_and(|targets| targets.iter().any(|target| target == to))
    }

    pub(super) fn is_writable(relative: &str) -> bool {
        let style = relative.strip_suffix(&format!("/{}", PageSources::STYLE));
        let page_style = style.and_then(PageComponent::of).is_some();
        let document_style = relative
            .strip_prefix("client/shared/styles/")
            .is_some_and(|name| [name.ends_with("-page.css"), name.contains('/')] == [true, false]);
        let component_style = [
            relative.starts_with("client/components/page/"),
            relative.ends_with(&format!("/{}", PageSources::STYLE)),
        ] == [true; 2];
        [page_style, document_style, component_style].contains(&true)
    }

    fn targets(component: &PageComponent, from: &str) -> Vec<String> {
        let style = component.page_file(PageSources::STYLE) == from;
        let script = component.page_file(PageSources::SCRIPT) == from;
        let style_targets = [component.style(), component.document_style()];
        let mut targets: Vec<String> = style_targets.into_iter().filter(|_| style).collect();
        targets.extend(script.then(|| component.script()));
        targets
    }

    fn add_page(&mut self, planned: &[String], names: &BTreeSet<String>, view: &str) {
        match PageSources::read(&self.root, view, names) {
            Ok(page) => page
                .into_iter()
                .for_each(|page| self.add_sources(planned, &page)),
            Err(step) => self.manual_steps.push(step),
        }
    }

    fn add_sources(&mut self, planned: &[String], page: &PageSources) {
        let view = self.root.join(page.component.view());
        let taken = view.parent().is_some_and(Path::exists);
        self.manual_steps
            .extend(taken.then(|| Self::taken_step(page)));
        let free = !taken;
        let moved = self.add_markup(planned, page, [free, page.refused] == [true, true]);
        self.add_style(page, page.style.as_deref().filter(|_| free), moved);
        self.add_script(page, [free, page.script] == [true, true]);
    }

    fn add_markup(&mut self, planned: &[String], page: &PageSources, wanted: bool) -> bool {
        let step = Self::markup_step(planned, page).filter(|_| wanted);
        let moved = [wanted, step.is_none()] == [true, true];
        self.manual_steps.extend(step);
        let changes = moved.then(|| Self::markup_changes(page));
        self.changes.extend(changes.into_iter().flatten());
        moved
    }

    fn markup_step(planned: &[String], page: &PageSources) -> Option<ManualStep> {
        let reasons = [
            (
                page.source.contains("<script"),
                "The page holds a state script, which the fix cannot move into a component.",
                format!(
                    "Move the page's markup and state into {} by hand, and place <{} /> in the page.",
                    page.component.view(),
                    page.component.tag()
                ),
            ),
            (
                planned.contains(&page.view),
                "Other fixes change this page first.",
                String::from(
                    "Run 'ty migrate fix --write' again to move the page's markup into a component.",
                ),
            ),
        ];
        reasons
            .into_iter()
            .find(|(found, _, _)| *found)
            .map(|(_, problem, action)| {
                let problem = format!("ty bundle refuses this page's markup (TY1309). {problem}");
                ManualStep::new(&page.view, problem, action)
            })
    }

    fn markup_changes(page: &PageSources) -> [FileChange; 2] {
        let view = page.component.view();
        let tag = page.component.tag();
        let note = format!(
            "Moved the page's markup into {view} and placed <{tag} /> in the page; a page only composes components (TY1309, ADR 0029)."
        );
        [
            FileChange::whole(&view, String::new(), &page.source, &note),
            FileChange::whole(
                &page.view,
                page.source.clone(),
                &format!("<{tag} />\n"),
                &note,
            ),
        ]
    }

    fn add_style(&mut self, page: &PageSources, style: Option<&str>, moved: bool) {
        let split = style.map(|style| (style, PageUpgrade::split_styles(style)));
        match split {
            Some((style, Some(split))) => self.add_split(page, style, &split, moved),
            Some((_, None)) => self.manual_steps.push(Self::style_step(
                page,
                "The fix cannot read this stylesheet.",
            )),
            None => {}
        }
    }

    fn add_split(&mut self, page: &PageSources, style: &str, split: &SplitStyles, moved: bool) {
        let document = !split.foundation.trim().is_empty();
        let component = !split.component.trim().is_empty();
        let only_document = [document, component] == [true, false];
        let stranded = [moved, only_document] == [false, false];
        let primary = [
            (page.component.style(), &split.component),
            (page.component.document_style(), &split.foundation),
        ][usize::from(document)]
        .clone();
        let taken = self.root.join(&primary.0).exists();
        let blocked = [
            (
                stranded,
                "Its rules style markup that stays in other components, so the fix cannot tell where they belong.",
            ),
            (taken, "The file it would move to already exists."),
        ];
        let step = blocked.into_iter().find(|(found, _)| *found);
        self.manual_steps
            .extend(step.map(|(_, problem)| Self::style_step(page, problem)));
        let free = step.is_none();
        let from = page.component.page_file(PageSources::STYLE);
        let note = String::from(
            "Moved the page's stylesheet: rules a global foundation stylesheet accepts go to client/shared/styles and tac.styles, and the rest go beside the page's component, whose styles are scoped to it (ADR 0108).",
        );
        self.changes
            .extend(free.then(|| FileChange::whole(&from, String::from(style), primary.1, &note)));
        self.moves.extend(free.then(|| FileMove {
            from: from.clone(),
            to: primary.0.clone(),
        }));
        let both = [free, document, component] == [true; 3];
        self.changes.extend(both.then(|| {
            FileChange::whole(
                &page.component.style(),
                String::new(),
                &split.component,
                &note,
            )
        }));
        let published = PageComponent::published(&page.component.document_style()).to_owned();
        self.styles.extend((free & document).then_some(published));
    }

    fn script_step(&self, page: &PageSources) -> Option<ManualStep> {
        let destination = page.component.script();
        let reasons = [
            (
                !self.single,
                String::from(
                    "A page no longer has a script, and tac.scripts would run it on every page of this app.",
                ),
            ),
            (
                self.root.join(&destination).exists(),
                format!("'{destination}' already exists."),
            ),
        ];
        reasons
            .into_iter()
            .find(|(found, _)| *found)
            .map(|(_, problem)| {
                ManualStep::new(
                    &page.component.page_file(PageSources::SCRIPT),
                    problem,
                    String::from("Move what the script does into a service under client/services/ that a component addresses (ADR 0030), or into client/shared/scripts/ listed in tac.scripts."),
                )
            })
    }

    fn add_script(&mut self, page: &PageSources, wanted: bool) {
        let step = wanted.then(|| self.script_step(page)).flatten();
        let free = [wanted, step.is_none()] == [true, true];
        self.manual_steps.extend(step);
        let destination = page.component.script();
        self.scripts
            .extend(free.then(|| PageComponent::published(&destination).to_owned()));
        self.moves.extend(free.then(|| FileMove {
            from: page.component.page_file(PageSources::SCRIPT),
            to: destination,
        }));
    }

    fn add_manifest(&mut self, planned: &[String]) {
        let manifest = ManifestMigration::MANIFEST;
        let listed = [self.styles.is_empty(), self.scripts.is_empty()] != [true, true];
        let exists = [
            self.root.join(manifest).exists(),
            planned.iter().any(|source| source == manifest),
        ]
        .contains(&true);
        let change = ([listed, exists] == [true, false]).then(|| self.manifest_change());
        let step = ([listed, exists] == [true, true]).then(|| self.manifest_step());
        self.changes.extend(change);
        self.manual_steps.extend(step);
    }

    fn manifest_change(&self) -> FileChange {
        let lists = [("scripts", &self.scripts), ("styles", &self.styles)];
        let tac: Map<String, Value> = lists
            .into_iter()
            .filter(|(_, paths)| !paths.is_empty())
            .map(|(name, paths)| (String::from(name), Value::from(paths.clone())))
            .collect();
        let manifest: Map<String, Value> = [(String::from("tac"), Value::Object(tac))]
            .into_iter()
            .collect();
        let text = serde_json::to_string_pretty(&SortedJson::map(&manifest)).unwrap_or_default();
        FileChange::whole(
            ManifestMigration::MANIFEST,
            String::new(),
            &format!("{text}\n"),
            "Listed the page's script in tac.scripts and its document styles in tac.styles.",
        )
    }

    fn manifest_step(&self) -> ManualStep {
        let listed = |kind: &str, paths: &[String]| {
            paths
                .iter()
                .map(|path| format!("'{path}' to tac.{kind}"))
                .collect::<Vec<_>>()
        };
        let additions = [
            listed("scripts", &self.scripts),
            listed("styles", &self.styles),
        ]
        .concat();
        ManualStep::new(
            ManifestMigration::MANIFEST,
            String::from(
                "The fix moved a page's script or document styles to client/shared, and it does not rewrite an existing manifest.",
            ),
            format!("Add {} in client/manifest.json.", additions.join(" and ")),
        )
    }

    fn taken_step(page: &PageSources) -> ManualStep {
        ManualStep::new(
            &page.view,
            format!(
                "ty bundle refuses what this page folder holds (TY1309), and '{}' already exists.",
                page.component.view().trim_end_matches("/tac.html")
            ),
            format!(
                "Move the page's markup and companions into a new component by hand, and place its tag in the page; the fix would use <{} />.",
                page.component.tag()
            ),
        )
    }

    fn style_step(page: &PageSources, problem: &str) -> ManualStep {
        ManualStep::new(
            &page.component.page_file(PageSources::STYLE),
            format!("A page folder holds only tac.html (TY1309). {problem}"),
            format!(
                "Move document colours, fonts and resets to client/shared/styles/ and list the file in tac.styles; move the rest beside the component that holds its markup, such as {}.",
                page.component.style()
            ),
        )
    }
}
