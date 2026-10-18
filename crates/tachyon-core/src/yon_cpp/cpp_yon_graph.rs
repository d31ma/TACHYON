use super::{CppYonController, CppYonErrors, CppYonProvider};
use crate::Failure;
use crate::tac_cpp::{CppClass, CppFailure, CppLifetime, CppProgram, CppServiceGraph};
use crate::yon::plan::Scope;
use std::path::Path;

pub(crate) struct CppYonGraph {
    pub(crate) providers: Vec<CppYonProvider>,
}

impl CppYonGraph {
    const FOLDERS: &'static [&'static str] =
        &["server/services", "server/helpers", "server/converters"];
    const TRANSIENT_EVENTS: &'static str = "A Yon C++ class with [[OnEvent]], [[OnStart]] or [[OnStop]] runs with objects that live for the whole request, so no class it works with may be [[Lifetime(\"transient\")]]. Use \"request\" or \"singleton\"";

    pub(crate) fn read(
        root: &Path,
        controller: &CppYonController,
        models: &CppProgram,
        errors: &CppYonErrors,
    ) -> Result<Self, Failure> {
        let mut graph = CppServiceGraph::within(root, Self::FOLDERS, Self::missing)?;
        Self::add_handler_needs(&mut graph, errors)?;
        Self::add_needs(&mut graph, &controller.path, &controller.file.class)?;
        let mut providers = graph
            .finish()
            .iter()
            .map(|service| CppYonProvider::service(service, models))
            .collect::<Result<Vec<_>, _>>()?;
        providers.extend(Self::handler_providers(errors, models)?);
        providers.push(CppYonProvider::root(controller, models)?);
        let graph = Self { providers };
        graph.check_captures()?;
        graph.check_transient()?;
        Ok(graph)
    }

    pub(crate) fn located(&self) -> String {
        self.providers
            .iter()
            .map(|provider| provider.lowered.as_str())
            .collect()
    }

    fn add_handler_needs(
        graph: &mut CppServiceGraph<'_>,
        errors: &CppYonErrors,
    ) -> Result<(), Failure> {
        errors
            .handlers
            .iter()
            .try_for_each(|handler| Self::add_needs(graph, &handler.path, &handler.file.class))
    }

    fn handler_providers(
        errors: &CppYonErrors,
        models: &CppProgram,
    ) -> Result<Vec<CppYonProvider>, Failure> {
        errors
            .handlers
            .iter()
            .map(|handler| CppYonProvider::handler(handler, models))
            .collect()
    }

    fn add_needs(
        graph: &mut CppServiceGraph<'_>,
        path: &Path,
        class: &CppClass,
    ) -> Result<(), Failure> {
        for injection in CppLifetime::injections(class) {
            let at = class
                .members
                .iter()
                .find(|member| member.name == injection.field)
                .map_or(0, |member| member.name_at);
            graph.add_need(path, at, &injection.provider)?;
        }
        Ok(())
    }

    fn missing(path: &Path, at: usize, needed: &str) -> Failure {
        CppFailure::coded(
            CppYonController::CODE,
            path,
            at,
            &format!(
                "A C++ [[Needs]] field names a C++ [[Service]] in server/services, a C++ [[Helper]] in server/helpers or a C++ [[Converter]] in server/converters, and there is no {needed}.hpp in any of them. Write the class in its own file there, named after the class"
            ),
        )
    }

    fn check_captures(&self) -> Result<(), Failure> {
        let mut requesting: Vec<&str> = Vec::new();
        for provider in &self.providers {
            let captured = provider
                .needs
                .iter()
                .find(|need| requesting.contains(&need.as_str()));
            Self::refuse_capture(provider, captured)?;
            let reaches = [provider.scope == Scope::Request, captured.is_some()].contains(&true);
            requesting.extend(reaches.then_some(provider.name.as_str()));
        }
        Ok(())
    }

    fn refuse_capture(provider: &CppYonProvider, captured: Option<&String>) -> Result<(), Failure> {
        captured
            .filter(|_| provider.scope == Scope::Singleton)
            .map_or(Ok(()), |name| Err(Self::captured(provider, name)))
    }

    fn captured(singleton: &CppYonProvider, needed: &str) -> Failure {
        CppFailure::coded(
            CppYonController::CODE,
            &singleton.path,
            singleton.at,
            &format!(
                "Singleton dependency graph captures a request-scoped object through {needed}: {name} is a singleton, which is shared, so it cannot keep an object that lives for one request. Make {needed} and the classes it needs singletons, or give {name} [[Lifetime(\"request\")]]",
                name = singleton.name
            ),
        )
    }

    fn check_transient(&self) -> Result<(), Failure> {
        let hooked = self
            .providers
            .iter()
            .any(|provider| !provider.hooks.is_empty());
        let transient = self
            .providers
            .iter()
            .find(|provider| provider.scope == Scope::Transient);
        let refused = transient.filter(|_| hooked);
        refused.map_or(Ok(()), |provider| {
            Err(CppFailure::coded(
                CppYonController::CODE,
                &provider.path,
                provider.at,
                Self::TRANSIENT_EVENTS,
            ))
        })
    }
}
