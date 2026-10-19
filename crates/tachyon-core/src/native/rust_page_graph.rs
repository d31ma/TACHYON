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
    service_scopes: super::rust_service_scopes::RustServiceScopes,
    utilities: BTreeSet<String>,
    remotes: Vec<crate::events::EventModule>,
}

impl RustPageGraph {
    pub(super) const MODULE_START: &'static str = "mod TachyonRustServices {\nuse super::*;\n";

    pub(super) fn source(inputs: &[&NativeCompanionInput]) -> Result<String, Failure> {
        let graph = Self::read(inputs)?;
        graph.validate(inputs)?;
        graph.emit(inputs)
    }

    fn read(inputs: &[&NativeCompanionInput]) -> Result<Self, Failure> {
        let input = inputs[0];
        let captured = std::fs::read_to_string(&input.source)
            .map_err(|error| super::host::native_tool_failure(1605, &error.to_string()))?;
        let relative = input
            .source
            .strip_prefix(&input.snapshot_root)
            .unwrap_or(&input.source);
        crate::annotation_layout::NativeWebApis::refuse_preferences(relative, &captured)?;
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
            service_scopes: super::rust_service_scopes::RustServiceScopes::default(),
            utilities: BTreeSet::new(),
            remotes: Vec::new(),
        };
        for (path, source) in units {
            graph.add_unit(&path, &source, &input.snapshot_root)?;
        }
        let lowered = crate::behavior::lower(&input.source, &graph.source, "tac")?;
        graph.source = lowered.unwrap_or(graph.source);
        graph.validate_application_constructors()?;
        graph.arcs.extend(
            graph
                .services
                .values()
                .filter(|service| service.application)
                .map(|service| service.name.clone()),
        );
        graph.reachable(inputs)?;
        Ok(graph)
    }

    fn validate_application_constructors(&self) -> Result<(), Failure> {
        self.services.values().try_for_each(|service| {
            super::native_application_capture::NativeApplicationCapture::constructor(
                service,
                &self.source,
            )
        })
    }

    fn reachable(&mut self, inputs: &[&NativeCompanionInput]) -> Result<(), Failure> {
        let names = self.services.keys().cloned().collect::<BTreeSet<_>>();
        let mut pending = inputs
            .iter()
            .map(|input| input.class.clone())
            .collect::<BTreeSet<_>>();
        pending.extend(
            self.services
                .values()
                .filter(|service| service.application)
                .map(|service| service.name.clone()),
        );
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
        super::native_application_capture::NativeApplicationCapture::dependencies(
            service,
            &self.services,
            &dependencies,
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
        crate::route_types::Converters::require_rust_mappings(root, path, source)?;
        let cleaned = self.unit_source(path, source, root)?;
        crate::behavior::NativeStorage::refuse_builtins(path, &cleaned)?;
        crate::annotation_layout::NativeWebApis::refuse_with_models(path, &cleaned, root)?;
        self.utilities.extend(Self::utility_name(path, source));
        let cleaned = self.unit_imports(path, &cleaned)?;
        self.source.push_str(&cleaned);
        self.source.push('\n');
        Ok(())
    }

    fn unit_source(
        &mut self,
        path: &std::path::Path,
        source: &str,
        root: &std::path::Path,
    ) -> Result<String, Failure> {
        if path.starts_with("client/services") {
            let mut service = RustServicePlan::read(path, source)?;
            super::native_application_capture::NativeApplicationCapture::validate(&service)?;
            Self::describe(&mut service, root)?;
            let cleaned = service.source.clone();
            self.services.insert(service.name.clone(), service);
            Ok(cleaned)
        } else {
            self.remotes
                .extend(Self::remote_events(path, source, root)?);
            crate::behavior::ServiceRoles::without_roles(path, source)
        }
    }

    fn remote_events(
        path: &std::path::Path,
        source: &str,
        root: &std::path::Path,
    ) -> Result<Option<crate::events::EventModule>, Failure> {
        let mut events = crate::events::extract(path, source, "tac")?;
        events
            .as_mut()
            .map(|module| crate::events::describe(module, path, root, "client"))
            .transpose()?;
        Ok(events)
    }

    fn unit_imports(&mut self, path: &std::path::Path, source: &str) -> Result<String, Failure> {
        match (Self::utility_name(path, source), self.service_name(path)) {
            (Some(name), _) => super::rust_imports::RustImports::scoped(path, source, &name),
            (None, Some(name)) => self.service_scopes.mark(path, source, &name),
            (None, None) => self.imports.take(path, source),
        }
    }

    fn service_name(&self, path: &std::path::Path) -> Option<String> {
        self.services
            .values()
            .find(|service| service.path == path)
            .map(|service| service.name.clone())
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
            .filter(|input| !input.application_provider)
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
        super::native_application_capture::NativeApplicationCapture::projection(
            service,
            roots.contains(service.name.as_str()),
        )?;
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
            .map(|name| Ok((name.clone(), self.definition(&self.services[name])?)))
            .collect::<Result<Vec<_>, Failure>>()?;
        let tables = inputs
            .iter()
            .filter(|input| !input.application_provider)
            .map(|input| Ok((input.class.clone(), self.table(input)?)))
            .collect::<Result<Vec<_>, Failure>>()?;
        let services = self
            .service_scopes
            .wrap(&self.source, &[definitions, tables].concat())?;
        let mut imports = BTreeSet::new();
        let models = super::registry::models_source(
            crate::project::NativeCompanion::Rust,
            &inputs[0].snapshot_root,
            &mut imports,
        )?;
        let dispatch = self.dispatch(inputs)?;
        let application = self.application_startup();
        let paths = super::rust_local_paths::RustLocalPaths::read(
            &inputs[0].source,
            &self.source,
            &models,
            &self.utilities,
        )?;
        let models = paths.source(&models)?;
        Ok(format!(
            "#[allow(non_snake_case)]\n{}{}\n{}\n{models}\n{application}\n}}\n{dispatch}",
            Self::MODULE_START,
            self.imports
                .source()
                .lines()
                .filter_map(|line| paths.import(line))
                .collect::<Vec<_>>()
                .join("\n"),
            paths.source(&services)?
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
            "pub(super) fn tac_definition_{name}() -> NativeServiceDefinition<{storage}> {{ NativeServiceDefinition {{ name: {name:?}, prototype: {}, application: {}, create: |scope| Ok({value}), start: |value| {start}, stop: |value| {stop} }} }}\n",
            service.prototype, service.application
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
        crate::route_types::Converters::require_rust_events(
            &input.snapshot_root,
            &service.path,
            service.events.as_ref(),
        )?;
        let mut events = service.prepared_events()?;
        events.entries.push_str(&self.held_streams(service)?);
        let _ = writeln!(
            events.entries,
            "(\"__graph_start\", TacMember::Hook(false, {{ let scope = scope.clone(); Box::new(move || scope.start_root({:?}, {}).map_err(|_| \"Native service startup failed\")) }})),",
            service.name, service.prototype
        );
        let obtained = format!("scope.obtain_root(&tac_definition_{}())?", service.name);
        let instance = Self::root_instance(&obtained, self.arcs.contains(&service.name));
        crate::route_types::Converters::require_rust_owner(
            &input.snapshot_root,
            &self.source,
            &service.path,
            &events.excluded,
        )?;
        crate::companion::RustGraph::table(
            &self.source,
            &service.path.to_string_lossy(),
            &instance,
            (&events.excluded, &events.entries),
        )
    }

    fn held_streams(&self, service: &RustServicePlan) -> Result<String, Failure> {
        super::rust_held_streams::RustHeldStreams::of_remotes(
            &self.remotes,
            &service.source,
            &service.name,
        )
        .map_err(|message| {
            super::host::native_tool_failure(
                1405,
                &format!("{}: {message}", service.path.display()),
            )
        })
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
            .filter(|input| !input.application_provider)
            .map(|input| &input.class)
            .collect::<BTreeSet<_>>();
        self.initializers
            .keys()
            .map(|name| &self.services[name])
            .filter(|service| !roots.contains(&service.name))
            .try_for_each(RustServicePlan::refuse_unowned_events)?;
        for input in inputs.iter().filter(|input| !input.application_provider) {
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
            "fn tac_service_members(service: &str) -> Option<Vec<(&'static str, TacMember)>> {{ let activation = NativePageServices::activation()?; let result: Result<Vec<(&'static str, TacMember)>, String> = match service {{ {entries}_ => return None }}; match result {{ Ok(members) => Some(members), Err(failure) => {{ activation.fail(failure); None }} }} }}\nfn tac_service_required(service: &str) -> &'static [&'static str] {{ match service {{ {required}_ => &[] }} }}\n#[unsafe(no_mangle)]\npub extern \"C\" fn tac_native_application_start() -> bool {{ NativeApplicationServices::start(TachyonRustServices::tac_application_providers).is_ok() }}\n#[unsafe(no_mangle)]\npub extern \"C\" fn tac_native_application_shutdown() {{ NativeApplicationServices::host_shutdown(); }}\n"
        ))
    }

    fn application_startup(&self) -> String {
        let providers = self
            .services
            .values()
            .filter(|service| service.application)
            .map(|service| format!("let _ = scope.obtain(&tac_definition_{}())?;", service.name))
            .collect::<Vec<_>>()
            .join("\n");
        let callers = self
            .services
            .values()
            .filter(|service| service.application)
            .map(|service| self.application_caller(&service.name))
            .collect::<String>();
        format!(
            "pub(super) fn tac_application_providers(scope: &NativeServiceScope) -> ::core::result::Result<(), ::std::string::String> {{ {providers} Ok(()) }}\n{callers}"
        )
    }

    fn application_caller(&self, name: &str) -> String {
        let borrowed = if self.arcs.contains(name) {
            "&*value"
        } else {
            "&*value.borrow()"
        };
        format!(
            "pub(super) fn tac_with_application_{name}(work: impl FnOnce(&{name})) -> ::core::result::Result<(), ::std::string::String> {{ NativeApplicationServices::obtain(&tac_definition_{name}()).map(|value| work({borrowed})) }}\n"
        )
    }
}
