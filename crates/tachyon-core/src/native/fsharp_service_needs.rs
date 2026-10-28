use super::fsharp_delegate_bases::{FsharpDelegateBase, FsharpDelegateBases};
use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::behavior::ServiceRoles;
use crate::fsharp_source::{FsharpConstructorProps, FsharpSource, FsharpView};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct FsharpServiceNeeds {
    instances: BTreeMap<String, String>,
    per_component: BTreeSet<String>,
    needs: BTreeMap<String, Vec<String>>,
    mentions: BTreeMap<String, BTreeSet<String>>,
    sources: BTreeMap<String, (String, String)>,
    namespaces: BTreeMap<String, String>,
    bases: BTreeMap<String, Vec<FsharpDelegateBase>>,
    roots: BTreeMap<String, PathBuf>,
}

impl FsharpServiceNeeds {
    const CODE: u16 = 1406;
    const NAMESPACE: &'static str = "TachyonService";
    const TAC: &'static str = "tac";
    pub(super) const PAGE_NAMESPACE: &'static str = "TachyonServiceFsharp";

    pub(super) fn ordered<'a>(
        selected: &[&'a NativeCompanionInput],
    ) -> Result<(Self, Vec<&'a NativeCompanionInput>), Failure> {
        let mut needs = Self::default();
        needs.add_all(selected)?;
        needs.find_needs();
        needs.refuse_unknown_bases()?;
        needs.refuse_per_component()?;
        needs.refuse_circles()?;
        let ordered = needs.order(selected);
        needs.place(&ordered);
        Ok((needs, ordered))
    }

    pub(super) fn argument(&self, type_name: &str) -> Option<String> {
        let instance = self.instances.get(type_name)?;
        let namespace = self.namespaces.get(type_name)?;
        Some(format!(
            "(global::{namespace}.{type_name})Tac.Need({instance:?})"
        ))
    }

    pub(super) fn with_bases(&self, assembled: &str) -> String {
        FsharpDelegateBases::filled(assembled, &self.instances)
    }

    pub(super) fn is_needed(&self, class: &str) -> bool {
        self.needs.values().flatten().any(|needed| needed == class)
    }

    pub(super) fn aliases(&self, class: &str) -> String {
        self.mentions
            .get(class)
            .into_iter()
            .flatten()
            .map(|needed| self.alias(needed))
            .collect()
    }

    fn alias(&self, needed: &str) -> String {
        let namespace = self.namespaces.get(needed).map_or("", String::as_str);
        format!("\ntype {needed} = {namespace}.{needed}\n")
    }

    fn add_all(&mut self, selected: &[&NativeCompanionInput]) -> Result<(), Failure> {
        for item in selected.iter().filter(|item| Self::is_fsharp(item)) {
            self.add(item)?;
        }
        Ok(())
    }

    fn is_fsharp(item: &NativeCompanionInput) -> bool {
        FsharpSource::is_fsharp(&item.source)
    }

    fn add(&mut self, item: &NativeCompanionInput) -> Result<(), Failure> {
        let source = super::registry::read_captured_source(&item.source)?;
        let relative = item
            .source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(&item.source)
            .to_string_lossy()
            .replace('\\', "/");
        let view = FsharpView::tac(Path::new(&relative), &source);
        let roles = ServiceRoles::read(view.path(), view.text())?;
        let needs_all = FsharpConstructorProps::needs_all(&source);
        self.instances
            .insert(item.class.clone(), item.service.clone());
        self.per_component.extend(
            [roles.prototype, needs_all]
                .contains(&true)
                .then(|| item.class.clone()),
        );
        self.add_bases(item, &relative, &source);
        self.sources.insert(item.class.clone(), (relative, source));
        Ok(())
    }

    fn add_bases(&mut self, item: &NativeCompanionInput, relative: &str, source: &str) {
        let assembled = crate::annotation_modules::assemble(
            &item.snapshot_root,
            Path::new(relative),
            crate::without_bom(source),
            Self::TAC,
        );
        let assembled = assembled.ok().flatten();
        let bases = FsharpDelegateBases::read(assembled.as_deref().unwrap_or(source), relative);
        self.bases.insert(item.class.clone(), bases);
        self.roots
            .insert(item.class.clone(), item.snapshot_root.clone());
    }

    fn find_needs(&mut self) {
        for (class, (relative, source)) in &self.sources {
            let parameters = FsharpConstructorProps::read(Path::new(relative), source, class);
            let bases = self.page_bases(class);
            let needed: Vec<String> = parameters
                .into_iter()
                .map(|parameter| parameter.type_name)
                .filter(|type_name| self.instances.contains_key(type_name))
                .chain(bases.iter().cloned())
                .collect();
            self.needs.insert(class.clone(), needed);
            let mut mentioned = self.mentioned(class, source);
            mentioned.extend(bases);
            self.mentions.insert(class.clone(), mentioned);
        }
    }

    fn page_bases(&self, class: &str) -> Vec<String> {
        self.bases
            .get(class)
            .into_iter()
            .flatten()
            .map(|base| base.kind.clone())
            .filter(|kind| self.instances.contains_key(kind))
            .collect()
    }

    fn refuse_unknown_bases(&self) -> Result<(), Failure> {
        let unknown = self
            .bases
            .iter()
            .flat_map(|(class, bases)| bases.iter().map(move |base| (class, base)))
            .find(|(_, base)| !self.instances.contains_key(&base.kind));
        unknown.map_or(Ok(()), |(class, base)| Err(self.unknown_base(class, base)))
    }

    fn unknown_base(&self, class: &str, base: &FsharpDelegateBase) -> Failure {
        let root = self
            .roots
            .get(class)
            .map_or(Path::new(""), PathBuf::as_path);
        let authored = std::fs::read_to_string(root.join(&base.path)).unwrap_or_default();
        let at = authored.find(&base.name).unwrap_or(0);
        Failure::one(crate::failure::diagnostic(
            Self::CODE,
            format!(
                "F# Tac Delegate {} needs {}, but the Needs property of an F# Tac Delegate names its base: one service that this page uses (ADR 0112). Give the Delegate other values through method parameters, or let the service that holds it set them.",
                base.delegate, base.kind
            ),
            None,
            crate::failure::source_span(&base.path, at, at + base.name.len()),
        ))
    }

    fn mentioned(&self, class: &str, source: &str) -> BTreeSet<String> {
        source
            .split(Self::separates)
            .filter(|word| self.names_another(class, word))
            .map(str::to_owned)
            .collect()
    }

    fn separates(letter: char) -> bool {
        [letter.is_alphanumeric(), letter == '_'] == [false, false]
    }

    fn names_another(&self, class: &str, word: &str) -> bool {
        [word != class, self.instances.contains_key(word)] == [true, true]
    }

    fn refuse_per_component(&self) -> Result<(), Failure> {
        for (class, needed) in &self.needs {
            let shared = needed
                .iter()
                .find(|needed| self.per_component.contains(*needed));
            shared.map_or(Ok(()), |needed| {
                Err(self.per_component_failure(class, needed))
            })?;
        }
        Ok(())
    }

    fn per_component_failure(&self, class: &str, needed: &str) -> Failure {
        self.refused(
            class,
            needed,
            &format!(
                "F# service {class} cannot need {needed}: {needed} has one instance for each component, because it is Lifetime prototype or NeedsAll, so one property cannot hold it. Make {needed} a singleton without properties, or pass the value to a method of {class}."
            ),
        )
    }

    fn refuse_circles(&self) -> Result<(), Failure> {
        for (class, needed) in &self.needs {
            let circle = needed
                .iter()
                .find(|needed| self.reached(needed).contains(class));
            circle.map_or(Ok(()), |needed| Err(self.circle_failure(class, needed)))?;
        }
        Ok(())
    }

    fn circle_failure(&self, class: &str, needed: &str) -> Failure {
        self.refused(
            class,
            needed,
            &format!(
                "F# services {class} and {needed} need each other in a circle. Break the circle with an event: one service publishes it and the other listens with OnEvent."
            ),
        )
    }

    fn reached(&self, from: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::from([from.to_owned()]);
        for _ in 0..self.needs.len() {
            seen = self.widened(&seen);
        }
        seen
    }

    fn widened(&self, seen: &BTreeSet<String>) -> BTreeSet<String> {
        let mut next = seen.clone();
        for class in seen {
            next.extend(self.needs.get(class).into_iter().flatten().cloned());
        }
        next
    }

    fn depth(&self, class: &str) -> usize {
        let mut depth = 0;
        let mut level = BTreeSet::from([class.to_owned()]);
        for _ in 0..self.mentions.len() {
            level = self.mentioned_by(&level);
            depth += usize::from(!level.is_empty());
        }
        depth
    }

    fn mentioned_by(&self, level: &BTreeSet<String>) -> BTreeSet<String> {
        level
            .iter()
            .flat_map(|class| self.mentions.get(class).into_iter().flatten().cloned())
            .collect()
    }

    fn order<'a>(&self, selected: &[&'a NativeCompanionInput]) -> Vec<&'a NativeCompanionInput> {
        let mut ordered = selected.to_vec();
        ordered.sort_by_key(|item| self.depth(&item.class));
        ordered
    }

    fn place(&mut self, ordered: &[&NativeCompanionInput]) {
        for (index, item) in ordered.iter().enumerate() {
            self.namespaces
                .insert(item.class.clone(), Self::namespace(index, item));
        }
    }

    fn namespace(index: usize, item: &NativeCompanionInput) -> String {
        let own = format!("{}{index}", Self::NAMESPACE);
        let shared = Self::PAGE_NAMESPACE.to_owned();
        [own, shared]
            .into_iter()
            .nth(usize::from(item.web_assembly))
            .unwrap_or_default()
    }

    fn refused(&self, class: &str, needed: &str, message: &str) -> Failure {
        let (relative, source) = self
            .sources
            .get(class)
            .map_or(("", ""), |(relative, source)| {
                (relative.as_str(), source.as_str())
            });
        let at = source.find(needed).unwrap_or(0);
        Failure::one(crate::failure::diagnostic(
            Self::CODE,
            message,
            None,
            crate::failure::source_span(relative, at, at + needed.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::FsharpServiceNeeds;
    use crate::native::registry::NativeCompanionInput;
    use crate::project::NativeCompanion;
    use std::path::Path;

    struct Page;

    impl Page {
        const ZETA: &'static str =
            "[<Service>]\ntype ZetaService() =\n    member val Count = 0 with get, set\n";
        const ALPHA: &'static str = "[<Service>]\ntype AlphaService() =\n    [<Needs>]\n    member val Zeta : ZetaService = Unchecked.defaultof<ZetaService> with get, set\n    member val Total = 0 with get, set\n";
        const BACK: &'static str = "[<Service>]\ntype ZetaService(alpha: AlphaService) =\n    member val Count = 0 with get, set\n";
        const QUOTE: &'static str = "[<Service>]\ntype QuoteService() =\n    [<Needs>]\n    member val TaxDelegate : TaxDelegate = Unchecked.defaultof<TaxDelegate> with get, set\n    member val Total = 0 with get, set\n";
        const TAX: &'static str =
            "[<Service>]\ntype TaxService() =\n    member val Asked = 0 with get, set\n";
        const DELEGATE: &'static str = "[<Delegate>]\ntype TaxDelegate() =\n    [<Needs>]\n    member val TaxService : TaxService = Unchecked.defaultof<TaxService> with get, set\n\n    member this.Rate() : int = this.TaxService.Asked\n";

        fn input(root: &Path, class: &str, source: &str) -> NativeCompanionInput {
            let path = root.join(format!("client/services/{class}.fs"));
            std::fs::create_dir_all(path.parent().expect("parent")).expect("services");
            std::fs::write(&path, source).expect("service source");
            let name = class.trim_end_matches("Service").to_ascii_lowercase();
            NativeCompanionInput {
                snapshot_root: root.to_path_buf(),
                language: NativeCompanion::CSharp,
                source: path,
                service: name,
                class: class.to_owned(),
                web_assembly: true,
                application_provider: false,
            }
        }

        fn inputs(root: &Path, zeta: &str) -> [NativeCompanionInput; 2] {
            [
                Self::input(root, "AlphaService", Self::ALPHA),
                Self::input(root, "ZetaService", zeta),
            ]
        }

        fn with_delegate(root: &Path, services: &[(&str, &str)]) -> Vec<NativeCompanionInput> {
            let path = root.join("client/delegates/TaxDelegate.fs");
            std::fs::create_dir_all(path.parent().expect("parent")).expect("delegates");
            std::fs::write(&path, Self::DELEGATE).expect("delegate source");
            services
                .iter()
                .map(|(class, source)| Self::input(root, class, source))
                .collect()
        }

        fn refusal(zeta: &str) -> String {
            let root = tempfile::tempdir().expect("project");
            let inputs = Self::inputs(root.path(), zeta);
            let selected: Vec<&NativeCompanionInput> = inputs.iter().collect();
            let Err(failure) = FsharpServiceNeeds::ordered(&selected) else {
                panic!("the page is refused");
            };
            let diagnostic = &failure.diagnostics()[0];
            assert_eq!(diagnostic.code.to_string(), "TY1406");
            assert_eq!(diagnostic.spans[0].file, "client/services/AlphaService.fs");
            diagnostic.message.clone()
        }
    }

    #[test]
    fn a_needed_service_comes_first_and_the_needing_service_names_it() {
        let root = tempfile::tempdir().expect("project");
        let inputs = Page::inputs(root.path(), Page::ZETA);
        let selected: Vec<&NativeCompanionInput> = inputs.iter().collect();
        let (needs, ordered) = FsharpServiceNeeds::ordered(&selected).expect("needs");
        let classes: Vec<&str> = ordered.iter().map(|item| item.class.as_str()).collect();
        assert_eq!(classes, ["ZetaService", "AlphaService"]);
        assert_eq!(
            needs.argument("ZetaService").as_deref(),
            Some("(global::TachyonServiceFsharp.ZetaService)Tac.Need(\"zeta\")")
        );
        assert!(needs.is_needed("ZetaService"));
        assert!(!needs.is_needed("AlphaService"));
        assert_eq!(
            needs.aliases("AlphaService"),
            "\ntype ZetaService = TachyonServiceFsharp.ZetaService\n"
        );
        assert_eq!(needs.aliases("ZetaService"), "");
    }

    #[test]
    fn the_page_shares_the_needed_service_and_the_table_asks_for_it() {
        let root = tempfile::tempdir().expect("project");
        let inputs = Page::inputs(root.path(), Page::ZETA);
        let registry = crate::native::registry::source(&inputs, NativeCompanion::CSharp)
            .expect("registry")
            .expect("source");
        let zeta = registry
            .split("namespace TachyonService0 {")
            .nth(1)
            .and_then(|rest| rest.split("namespace TachyonService1 {").next())
            .expect("needed service");
        assert!(zeta.contains("Tac.Keep(Instance);"), "{zeta}");
        assert!(
            zeta.contains("internal const bool Prototype = false;"),
            "{zeta}"
        );
        let alpha = registry
            .split("namespace TachyonService1 {")
            .nth(1)
            .expect("needing service");
        assert!(
            alpha.contains(
                "new AlphaService() { Zeta = (global::TachyonServiceFsharp.ZetaService)Tac.Need(\"zeta\") }"
            ),
            "{alpha}"
        );
        assert!(!alpha.contains("[\"zeta\"]"), "{alpha}");
        assert!(
            alpha.contains("internal const bool Prototype = true;"),
            "{alpha}"
        );
        assert_eq!(
            registry
                .matches("namespace rec TachyonServiceFsharp\n")
                .count(),
            1,
            "{registry}"
        );
        assert!(
            !registry.contains("namespace rec TachyonService1\n"),
            "{registry}"
        );
    }

    #[test]
    fn a_service_with_one_instance_for_each_component_cannot_be_needed() {
        for zeta in [
            Page::ZETA.replace(
                "[<Service>]\n",
                "[<Service>]\n[<Lifetime({\"value\":\"prototype\"})>]\n",
            ),
            Page::ZETA.replace("[<Service>]\n", "[<Service>]\n[<NeedsAll>]\n"),
        ] {
            let message = Page::refusal(&zeta);
            assert!(
                message.contains("ZetaService has one instance for each component"),
                "{message}"
            );
        }
    }

    #[test]
    fn services_that_need_each_other_are_refused() {
        let message = Page::refusal(Page::BACK);
        assert!(message.contains("need each other in a circle"), "{message}");
    }

    #[test]
    fn a_delegate_base_is_a_need_of_the_service_that_holds_the_delegate() {
        let root = tempfile::tempdir().expect("project");
        let inputs = Page::with_delegate(
            root.path(),
            &[("QuoteService", Page::QUOTE), ("TaxService", Page::TAX)],
        );
        let selected: Vec<&NativeCompanionInput> = inputs.iter().collect();
        let (needs, ordered) = FsharpServiceNeeds::ordered(&selected).expect("needs");
        let classes: Vec<&str> = ordered.iter().map(|item| item.class.as_str()).collect();
        assert_eq!(classes, ["TaxService", "QuoteService"]);
        assert!(needs.is_needed("TaxService"));
        assert!(!needs.is_needed("QuoteService"));
        let registry = crate::native::registry::source(&inputs, NativeCompanion::CSharp)
            .expect("registry")
            .expect("source");
        assert!(
            registry.contains(
                "member val TaxService : TaxService = TacDelegateBase.Need<TaxService>(\"tax\") with get, set"
            ),
            "{registry}"
        );
        assert!(
            !registry.contains("Unchecked.defaultof<TaxService>"),
            "{registry}"
        );
        assert!(
            registry.contains("TacDelegateBase.Needed = Tac.Need;"),
            "{registry}"
        );
        assert_eq!(
            registry
                .matches("public static class TacDelegateBase")
                .count(),
            1,
            "{registry}"
        );
        let tax = registry
            .split("namespace TachyonService0 {")
            .nth(1)
            .and_then(|rest| rest.split("namespace TachyonService1 {").next())
            .expect("base service");
        assert!(tax.contains("Tac.Keep(Instance);"), "{tax}");
    }

    #[test]
    fn a_delegate_base_that_the_page_does_not_use_is_refused() {
        let root = tempfile::tempdir().expect("project");
        let inputs = Page::with_delegate(root.path(), &[("QuoteService", Page::QUOTE)]);
        let selected: Vec<&NativeCompanionInput> = inputs.iter().collect();
        let Err(failure) = FsharpServiceNeeds::ordered(&selected) else {
            panic!("the page is refused");
        };
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1406");
        assert!(diagnostic.message.contains("names its base"), "{failure}");
        assert_eq!(diagnostic.spans[0].file, "client/delegates/TaxDelegate.fs");
    }

    #[test]
    fn a_delegate_whose_base_holds_it_is_a_circle() {
        let root = tempfile::tempdir().expect("project");
        let quote = Page::QUOTE.replace("QuoteService", "TaxService");
        let inputs = Page::with_delegate(root.path(), &[("TaxService", &quote)]);
        let selected: Vec<&NativeCompanionInput> = inputs.iter().collect();
        let Err(failure) = FsharpServiceNeeds::ordered(&selected) else {
            panic!("the page is refused");
        };
        assert!(
            failure.diagnostics()[0].message.contains("in a circle"),
            "{failure}"
        );
    }
}
