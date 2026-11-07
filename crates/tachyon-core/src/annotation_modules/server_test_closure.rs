use super::{Module, ModuleInput, owner_by_name};
use crate::Failure;
use crate::annotation_layout::{Token, Tokens, masked_source};
use crate::method_coverage::CallHooks;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct ServerTestClosure {
    owners: BTreeMap<String, BTreeSet<usize>>,
    pending: Vec<usize>,
    retained: BTreeSet<usize>,
}

impl ServerTestClosure {
    const REGISTRATIONS: [&'static str; 7] = [
        "OnEvent",
        "OnStart",
        "OnStop",
        "ErrorHandler",
        "IdentityProvider",
        "StreamRequest",
        "SocketRequest",
    ];
    const IMPORT: &'static str = "import";
    const DYNAMIC: [&'static str; 6] = [
        "reflect",
        "unsafe",
        "Gateway",
        "YonGateway",
        "YonGatewayClient",
        "BehaviorMethodPolicy",
    ];

    pub(super) fn discovery_coverage(
        entry: &Path,
        path: &Path,
        source: String,
    ) -> Result<String, Failure> {
        Self::coverage(entry, path, source)
    }

    pub(super) fn select(
        snapshot_root: &Path,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
    ) -> Result<(), Failure> {
        Self::wanted(input.entry)
            .then(|| Self::selected(snapshot_root, input, modules))
            .transpose()
            .map(|_| ())
    }

    fn wanted(entry: &Path) -> bool {
        [
            entry.starts_with("server/tests"),
            entry.extension().is_some_and(|extension| extension == "go"),
        ] == [true; 2]
    }

    fn coverage(entry: &Path, path: &Path, source: String) -> Result<String, Failure> {
        if Self::wanted(entry) {
            return Ok(source);
        }
        CallHooks::for_entry(entry, path, source)
    }

    fn selected(
        snapshot_root: &Path,
        input: ModuleInput<'_>,
        modules: &mut Vec<Module>,
    ) -> Result<(), Failure> {
        let mut closure = Self::new(modules)?;
        closure.references(input.entry, input.source, modules.len())?;
        closure.shared_sources(snapshot_root, modules.len())?;
        closure.registrations(modules)?;
        closure.complete(modules)?;
        let mut index = 0;
        modules.retain(|_| {
            let keep = closure.retained.contains(&index);
            index += 1;
            keep
        });
        modules.iter_mut().try_for_each(Self::record_coverage)
    }

    fn shared_sources(&mut self, snapshot_root: &Path, count: usize) -> Result<(), Failure> {
        for (path, source) in crate::handler::GoSharedCode::read(snapshot_root)? {
            self.references(&path, &source, count)?;
        }
        Ok(())
    }

    fn new(modules: &[Module]) -> Result<Self, Failure> {
        let names = owner_by_name(modules)?;
        let mut owners: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for (name, index) in names {
            owners
                .entry(format!("New{name}"))
                .or_default()
                .insert(index);
            owners
                .entry(format!("New{name}With"))
                .or_default()
                .insert(index);
            owners.entry(name).or_default().insert(index);
        }
        Ok(Self {
            owners,
            pending: Vec::new(),
            retained: BTreeSet::new(),
        })
    }

    fn registrations(&mut self, modules: &[Module]) -> Result<(), Failure> {
        for (index, module) in modules.iter().enumerate() {
            let code = masked_source(&module.path, &module.source)?;
            self.registration(index, &code);
        }
        Ok(())
    }

    fn registration(&mut self, index: usize, code: &str) {
        if code
            .lines()
            .filter_map(Self::marker)
            .any(|marker| Self::REGISTRATIONS.contains(&marker))
        {
            self.pending.push(index);
        }
    }

    fn marker(line: &str) -> Option<&str> {
        line.trim_start()
            .strip_prefix('@')
            .map(crate::go_source::GoMarkerName::read)
            .and_then(|name| name.rsplit("::").next())
    }

    fn complete(&mut self, modules: &[Module]) -> Result<(), Failure> {
        while let Some(index) = self.pending.pop() {
            self.visit(index, modules)?;
        }
        Ok(())
    }

    fn visit(&mut self, index: usize, modules: &[Module]) -> Result<(), Failure> {
        if !self.retained.insert(index) {
            return Ok(());
        }
        let module = &modules[index];
        self.references(&module.path, &module.source, modules.len())
    }

    fn references(&mut self, path: &Path, source: &str, count: usize) -> Result<(), Failure> {
        let code = masked_source(path, source)?;
        self.dynamic(path, source, &code, count)?;
        self.names(&code);
        self.annotations(source, &code);
        Ok(())
    }

    fn dynamic(
        &mut self,
        path: &Path,
        source: &str,
        code: &str,
        count: usize,
    ) -> Result<(), Failure> {
        let tokens = Tokens::read_source(path, source)?;
        let reflection = Self::dynamic_imports(&tokens);
        let selection = Self::words(code).any(|word| Self::DYNAMIC.contains(&word));
        if [reflection, selection].contains(&true) {
            self.pending.extend(0..count);
        }
        Ok(())
    }

    fn dynamic_imports(tokens: &Tokens<'_>) -> bool {
        tokens.items.iter().enumerate().any(|(at, token)| {
            Self::is_import(token).then(|| Self::dynamic_import(tokens, at + 1)) == Some(true)
        })
    }

    fn is_import(token: &Token<'_>) -> bool {
        [!token.literal, token.text == Self::IMPORT] == [true; 2]
    }

    fn dynamic_import(tokens: &Tokens<'_>, at: usize) -> bool {
        if tokens.items.get(at).is_some_and(|token| token.text == "(") {
            return Self::dynamic_import_group(tokens, at + 1);
        }
        Self::dynamic_import_spec(tokens, at)
    }

    fn dynamic_import_spec(tokens: &Tokens<'_>, at: usize) -> bool {
        let path = tokens
            .items
            .get(at)
            .map(|token| at + usize::from(!token.literal));
        path.and_then(|path| tokens.items.get(path))
            .is_none_or(Self::dynamic_import_path)
    }

    fn dynamic_import_group(tokens: &Tokens<'_>, at: usize) -> bool {
        let closed = tokens.items[at..].iter().any(|token| token.text == ")");
        let mut paths = tokens.items[at..]
            .iter()
            .take_while(|token| token.text != ")")
            .filter(|token| token.literal);
        [!closed, paths.any(Self::dynamic_import_path)].contains(&true)
    }

    fn dynamic_import_path(token: &Token<'_>) -> bool {
        let path = Self::import_path(token.text);
        [!token.literal, Self::reflection_path(path)].contains(&true)
    }

    fn reflection_path(path: &str) -> bool {
        [path.contains('\\'), ["reflect", "unsafe"].contains(&path)].contains(&true)
    }

    fn import_path(text: &str) -> &str {
        let text = text.trim();
        if let Some(raw) = text.strip_prefix('`') {
            return raw.split('`').next().unwrap_or_default();
        }
        text.trim_matches('"')
    }

    fn annotations(&mut self, source: &str, code: &str) {
        for (line, visible) in source.lines().zip(code.lines()) {
            self.annotation(line, visible);
        }
    }

    fn annotation(&mut self, line: &str, visible: &str) {
        if visible.trim_start().starts_with('@') {
            self.names(line);
        }
    }

    fn names(&mut self, source: &str) {
        for word in Self::words(source) {
            self.named(word);
        }
    }

    fn named(&mut self, word: &str) {
        if let Some(indexes) = self.owners.get(word) {
            self.pending.extend(indexes);
        }
    }

    fn words(source: &str) -> impl Iterator<Item = &str> {
        source.split(|character: char| {
            ![character.is_ascii_alphanumeric(), character == '_'].contains(&true)
        })
    }

    fn record_coverage(module: &mut Module) -> Result<(), Failure> {
        module.source = CallHooks::insert(&module.path, &module.source)?;
        Ok(())
    }
}
