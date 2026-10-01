use super::{registry::NativeCompanionInput, rust_service_plan::RustServicePlan};
use crate::Failure;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub(super) struct RustPageGraph {
    source: String,
    services: BTreeMap<String, RustServicePlan>,
    arcs: BTreeSet<String>,
    initializers: BTreeMap<String, String>,
    imports: super::rust_imports::RustImports,
    utilities: BTreeSet<String>,
}

impl RustPageGraph {
    pub(super) fn source(inputs: &[&NativeCompanionInput]) -> Result<String, Failure> {
        let graph = Self::read(inputs)?;
        graph.validate(inputs)?;
        graph.emit(inputs)
    }

    fn read(inputs: &[&NativeCompanionInput]) -> Result<Self, Failure> {
        let input = inputs[0];
        let captured = std::fs::read_to_string(&input.source)
            .map_err(|error| super::host::native_tool_failure(1605, &error.to_string()))?;
        let units = crate::annotation_modules::Module::native_units(
            &input.snapshot_root,
            &input.source,
            &captured,
        )?;
        let mut graph = Self {
            source: String::new(),
            services: BTreeMap::new(),
            arcs: BTreeSet::new(),
            initializers: BTreeMap::new(),
            imports: super::rust_imports::RustImports::default(),
            utilities: BTreeSet::new(),
        };
        for (path, source) in units {
            graph.add_unit(&path, &source, &input.snapshot_root)?;
        }
        let lowered = crate::behavior::lower(&input.source, &graph.source, "tac")?;
        graph.source = lowered.unwrap_or(graph.source);
        graph.reachable(inputs)?;
        Ok(graph)
    }

    fn reachable(&mut self, inputs: &[&NativeCompanionInput]) -> Result<(), Failure> {
        let names = self.services.keys().cloned().collect::<BTreeSet<_>>();
        let mut pending = inputs
            .iter()
            .map(|input| input.class.clone())
            .collect::<BTreeSet<_>>();
        while let Some(name) = pending.pop_first() {
            self.add_reachable(&name, &names, &mut pending)?;
        }
        Ok(())
    }

    fn add_reachable(
        &mut self,
        name: &str,
        names: &BTreeSet<String>,
        pending: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        if self.initializers.contains_key(name) {
            return Ok(());
        }
        let service = self.services.get(name).ok_or_else(|| {
            super::host::native_tool_failure(1406, "Native Rust required Service is not declared.")
        })?;
        let (initializer, dependencies) = crate::companion::RustGraph::instance(
            &self.source,
            &service.path.to_string_lossy(),
            name,
            names,
        )?;
        self.arcs.extend(dependencies.clone());
        pending.extend(dependencies);
        self.initializers.insert(name.to_owned(), initializer);
        Ok(())
    }

    fn add_unit(
        &mut self,
        path: &std::path::Path,
        source: &str,
        root: &std::path::Path,
    ) -> Result<(), Failure> {
        let cleaned = if path.starts_with("client/services") {
            let mut service = RustServicePlan::read(path, source)?;
            Self::describe(&mut service, root)?;
            let cleaned = service.source.clone();
            self.services.insert(service.name.clone(), service);
            cleaned
        } else {
            crate::behavior::ServiceRoles::without_roles(path, source)?
        };
        crate::behavior::NativeStorage::refuse_builtins(path, &cleaned)?;
        crate::annotation_layout::NativeWebApis::refuse(path, &cleaned)?;
        self.utilities.extend(Self::utility_name(path, source));
        let cleaned = self.imports.take(path, &cleaned)?;
        self.source.push_str(&cleaned);
        self.source.push('\n');
        Ok(())
    }

    fn utility_name(path: &std::path::Path, source: &str) -> Option<String> {
        crate::annotation_layout::UtilityLayout::is_utility(path)
            .then(|| crate::annotation_layout::ClassFileName::declared(path, source))
            .flatten()
    }

    fn describe(service: &mut RustServicePlan, root: &std::path::Path) -> Result<(), Failure> {
        service
            .events
            .as_mut()
            .map(|events| crate::events::describe(events, &service.path, root, "client"))
            .transpose()?;
        Ok(())
    }

    fn validate(&self, inputs: &[&NativeCompanionInput]) -> Result<(), Failure> {
        let roots = inputs
            .iter()
            .map(|input| input.class.as_str())
            .collect::<BTreeSet<_>>();
        self.initializers
            .keys()
            .map(|name| &self.services[name])
            .try_for_each(|service| {
                self.validate_service(service, &roots)?;
                self.validate_scope(service)
            })
    }

    fn validate_service(
        &self,
        service: &RustServicePlan,
        roots: &BTreeSet<&str>,
    ) -> Result<(), Failure> {
        let arc = self.arcs.contains(&service.name);
        let safe = crate::companion::RustGraph::supports_arc(
            &self.source,
            &service.path.to_string_lossy(),
            roots.contains(service.name.as_str()),
        )?;
        if arc && !safe {
            return Err(service.failure(&format!(
                    "Native Rust Arc provider '{}' has writable page fields or &mut self methods. Preserve its setters with an owned root, or author an immutable/interior-mutable provider; unsafe mutable Arc aliasing is not supported.",
                    service.name
                )));
        }
        Ok(())
    }

    fn validate_scope(&self, service: &RustServicePlan) -> Result<(), Failure> {
        let dependencies = crate::companion::RustGraph::dependencies(
            &self.source,
            &service.path.to_string_lossy(),
            &service.name,
        )?;
        let captures_prototype = dependencies
            .iter()
            .filter_map(|name| self.services.get(name))
            .any(|dependency| dependency.prototype);
        if !service.prototype && captures_prototype {
            return Err(service.failure("A page singleton cannot capture an activation prototype. Use an explicit compatible lifetime or handwritten composition; retaining the first component's prototype is not supported."));
        }
        Ok(())
    }

    fn emit(&self, inputs: &[&NativeCompanionInput]) -> Result<String, Failure> {
        let definitions = self
            .initializers
            .keys()
            .map(|name| self.definition(&self.services[name]))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        let tables = inputs
            .iter()
            .map(|input| self.table(input))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        let mut imports = BTreeSet::new();
        let models = super::registry::models_source(
            crate::project::NativeCompanion::Rust,
            &inputs[0].snapshot_root,
            &mut imports,
        )?;
        let dispatch = self.dispatch(inputs)?;
        let paths = super::rust_local_paths::RustLocalPaths::read(
            &inputs[0].source,
            &self.source,
            &models,
            &self.utilities,
        )?;
        let definitions = paths.source(&definitions)?;
        let tables = paths.source(&tables)?;
        let models = paths.source(&models)?;
        Ok(format!(
            "#[allow(non_snake_case)]\nmod TachyonRustServices {{\nuse super::*;\n{}\n{}\n{definitions}\n{tables}\n{models}\n}}\n{dispatch}",
            self.imports
                .source()
                .lines()
                .filter_map(|line| paths.import(line))
                .collect::<Vec<_>>()
                .join("\n"),
            paths.source(&self.source)?
        ))
    }

    fn definition(&self, service: &RustServicePlan) -> Result<String, Failure> {
        let name = &service.name;
        let arc = self.arcs.contains(name);
        let storage = Self::storage(name, arc);
        let instance = &self.initializers[name];
        let value = Self::value(instance, arc);
        let start = service.lifecycle(arc, false)?;
        let stop = service.lifecycle(arc, true)?;
        Ok(format!(
            "fn tac_definition_{name}() -> NativeServiceDefinition<{storage}> {{ NativeServiceDefinition {{ name: {name:?}, prototype: {}, create: |scope| Ok({value}), start: |value| {start}, stop: |value| {stop} }} }}\n",
            service.prototype
        ))
    }

    fn storage(name: &str, arc: bool) -> String {
        if arc {
            return format!("::std::sync::Arc<{name}>");
        }
        format!("::std::rc::Rc<::core::cell::RefCell<{name}>>")
    }

    fn value(instance: &str, arc: bool) -> String {
        if arc {
            return format!("::std::sync::Arc::new({instance})");
        }
        format!("::std::rc::Rc::new(::core::cell::RefCell::new({instance}))")
    }

    fn table(&self, input: &NativeCompanionInput) -> Result<String, Failure> {
        let service = self.services.get(&input.class).ok_or_else(|| {
            super::host::native_tool_failure(
                1406,
                "Native exposed Rust root has no validated Service definition.",
            )
        })?;
        let mut events = service.prepared_events()?;
        let _ = writeln!(
            events.entries,
            "(\"__graph_start\", TacMember::Hook(false, {{ let scope = scope.clone(); Box::new(move || scope.start_root({:?}, {}).map_err(|_| \"Native service startup failed\")) }})),",
            service.name, service.prototype
        );
        let obtained = format!("scope.obtain_root(&tac_definition_{}())?", service.name);
        let instance = Self::root_instance(&obtained, self.arcs.contains(&service.name));
        crate::companion::RustGraph::table(
            &self.source,
            &service.path.to_string_lossy(),
            &instance,
            (&events.excluded, &events.entries),
        )
    }

    fn root_instance(obtained: &str, arc: bool) -> String {
        if arc {
            return format!("::std::rc::Rc::new(::core::cell::RefCell::new({obtained}))");
        }
        obtained.to_owned()
    }

    fn dispatch(&self, inputs: &[&NativeCompanionInput]) -> Result<String, Failure> {
        let mut entries = String::new();
        let mut required = String::new();
        let roots = inputs
            .iter()
            .map(|input| &input.class)
            .collect::<BTreeSet<_>>();
        self.initializers
            .keys()
            .map(|name| &self.services[name])
            .filter(|service| !roots.contains(&service.name))
            .try_for_each(RustServicePlan::refuse_unowned_events)?;
        for input in inputs {
            let service = &self.services[&input.class];
            let _ = writeln!(
                entries,
                "{:?} => TachyonRustServices::tac_members_{}(&activation.scope),\n",
                input.service, service.name
            );
            let fields = service
                .required
                .iter()
                .map(|name| format!("{name:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(required, "{:?} => &[{fields}],", input.service);
        }
        Ok(format!(
            "fn tac_service_members(service: &str) -> Option<Vec<(&'static str, TacMember)>> {{ let activation = NativePageServices::activation()?; let result = match service {{ {entries}_ => return None }}; match result {{ Ok(members) => Some(members), Err(failure) => {{ activation.fail(failure); None }} }} }}\nfn tac_service_required(service: &str) -> &'static [&'static str] {{ match service {{ {required}_ => &[] }} }}\n"
        ))
    }
}
