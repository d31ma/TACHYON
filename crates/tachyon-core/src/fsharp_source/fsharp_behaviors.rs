use super::{BehaviorBody, FsharpView, MemberLine};
use crate::behavior::{DetachedCsharp, PlannedMethod, PlannedType};
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub(crate) struct FsharpBehaviors<'a> {
    view: &'a FsharpView,
    detached: &'a DetachedCsharp,
    lines: Vec<String>,
    extensions: BTreeMap<String, Vec<String>>,
}

impl<'a> FsharpBehaviors<'a> {
    const OWN_CONTEXT: &'static str = "this.behaviors";
    const GIVEN_CONTEXT: &'static str = "behaviors";
    const CONTEXT_PARAMETER: &'static str = "(behaviors: BehaviorContext)";
    const NO_PARAMETERS: &'static str = "()";
    const WRAPPER_INDENT: &'static str = "    ";

    pub(crate) fn program(
        view: &'a FsharpView,
        stripped: &str,
        detached: &'a DetachedCsharp,
    ) -> Result<String, String> {
        let mut behaviors = Self {
            view,
            detached,
            lines: stripped.split('\n').map(str::to_owned).collect(),
            extensions: BTreeMap::new(),
        };
        behaviors.rewrite()?;
        Ok(behaviors.joined())
    }

    fn rewrite(&mut self) -> Result<(), String> {
        self.give_constructors()?;
        self.lower_methods()
    }

    fn give_constructors(&mut self) -> Result<(), String> {
        for planned in self.detached.types.iter().filter(|ty| ty.declared) {
            self.give_constructor(planned)?;
        }
        Ok(())
    }

    fn lower_methods(&mut self) -> Result<(), String> {
        for method in &self.detached.methods {
            self.lower_method(method)?;
        }
        Ok(())
    }

    fn give_constructor(&mut self, planned: &PlannedType) -> Result<(), String> {
        let index = self.line_of(planned.line)?;
        let parameters =
            [Self::NO_PARAMETERS, Self::CONTEXT_PARAMETER][usize::from(planned.takes_context)];
        let line = &self.lines[index];
        let at = Self::type_name_end(line, &planned.name)
            .ok_or_else(|| format!("Cannot find the F# type {}", planned.name))?;
        self.lines[index] = format!("{}{parameters}{}", &line[..at], &line[at..]);
        Ok(())
    }

    fn type_name_end(line: &str, name: &str) -> Option<usize> {
        let found = line.find(&format!("type {name}"))?;
        Some(found + "type ".len() + name.len())
    }

    fn lower_method(&mut self, method: &PlannedMethod) -> Result<(), String> {
        let index = self.line_of(method.line)?;
        let declared = [method.declared, MemberLine::is_abstract(&self.lines[index])];
        if declared.contains(&true) {
            return self.write_declared(index, method);
        }
        self.wrap_written(index, method)
    }

    fn write_declared(&mut self, index: usize, method: &PlannedMethod) -> Result<(), String> {
        let receiver = self.receiver_for(&method.owner);
        let body = BehaviorBody::new(method, receiver).expression(None)?;
        self.lines[index] = MemberLine::concrete(&self.lines[index], &body)
            .ok_or_else(|| format!("Cannot read the F# abstract member {}", method.name))?;
        Ok(())
    }

    fn receiver_for(&self, owner: &str) -> &'static str {
        let given = self
            .detached
            .types
            .iter()
            .filter(|ty| ty.declared)
            .any(|ty| ty.name == owner);
        [Self::OWN_CONTEXT, Self::GIVEN_CONTEXT][usize::from(given)]
    }

    fn wrap_written(&mut self, index: usize, method: &PlannedMethod) -> Result<(), String> {
        let line = &self.lines[index];
        Self::instance_only(line, &method.name)?;
        let unreadable = || format!("Cannot read the F# member {}", method.name);
        let signature = MemberLine::signature(line, &method.name).ok_or_else(unreadable)?;
        let renamed = MemberLine::renamed(line, &method.name).ok_or_else(unreadable)?;
        let original = format!(
            "this.{}({})",
            MemberLine::hidden_name(&method.name),
            method.parameters.join(", ")
        );
        let body = BehaviorBody::new(method, Self::OWN_CONTEXT).expression(Some(&original))?;
        self.lines[index] = renamed;
        self.extensions
            .entry(method.owner.clone())
            .or_default()
            .push(format!(
                "{}member this.{}{signature} = {body}",
                Self::WRAPPER_INDENT,
                method.name
            ));
        Ok(())
    }

    fn instance_only(line: &str, name: &str) -> Result<(), String> {
        let instance = !MemberLine::is_static(line.trim_start());
        instance.then_some(()).ok_or_else(|| {
            format!(
                "The F# member {name} holds a behavior annotation, so it must be an instance member"
            )
        })
    }

    fn line_of(&self, shadow_line: usize) -> Result<usize, String> {
        self.view
            .source_line(shadow_line)
            .filter(|index| *index < self.lines.len())
            .ok_or_else(|| String::from("A behavior declaration has no F# line"))
    }

    fn joined(&self) -> String {
        let mut program = self.lines.join("\n");
        for (owner, members) in &self.extensions {
            let _ = write!(program, "\ntype {owner} with\n{}\n", members.join("\n"));
        }
        program
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpBehaviors;
    use crate::fsharp_source::{FsharpMarkers, FsharpView};
    use std::path::Path;

    const SERVICE: &str = "open System.Threading.Tasks\n\n[<Service>]\n[<CacheSettings({\"name\":\"catalog\",\"ttlMs\":30000,\"maxBytes\":65536,\"maxEntries\":256})>]\ntype CatalogService() =\n    let mutable reads = 0\n    member val behaviors = Behaviors.create(typeof<CatalogSetup>, \"catalog\", new BehaviorSignal()) with get, set\n\n    [<Cached>]\n    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> =\n        task {\n            reads <- reads + 1\n            return BehaviorJSON.From(reads)\n        }\n\n[<Converter>]\ntype ProductConverter =\n    [<Convert({\"fields\":[{\"source\":\"name\",\"target\":\"title\"}]})>]\n    abstract toSummary: product: BehaviorJSON -> BehaviorJSON\n\n[<Setup>]\ntype CatalogSetup() =\n    static member configure(execution: BehaviorExecution) : BehaviorAdapters = BehaviorAdapters()\n";

    fn lowered(path: &str, source: &str) -> (String, crate::behavior::DetachedCsharp) {
        let path = Path::new(path);
        let view = FsharpView::new(path, source);
        let detached = crate::behavior::detached_csharp(view.path(), view.text(), "yon")
            .unwrap_or_else(|failure| panic!("{failure}\n{}", view.text()))
            .unwrap_or_else(|| panic!("no behaviors"));
        let program = FsharpBehaviors::program(&view, &FsharpMarkers::strip(source), &detached)
            .unwrap_or_else(|message| panic!("{message}"));
        (program, detached)
    }

    #[test]
    fn a_cached_member_is_wrapped_and_an_abstract_converter_is_written() {
        let (program, detached) = lowered("server/services/CatalogService.fs", SERVICE);
        assert_eq!(
            program.lines().count(),
            SERVICE.lines().count() + 3,
            "{program}"
        );
        assert!(
            program.contains(
                "    member this.__tachyon_find(sku: BehaviorJSON) : Task<BehaviorJSON> ="
            ),
            "{program}"
        );
        assert!(program.contains("type CatalogService with\n    member this.find(sku: BehaviorJSON) : Task<BehaviorJSON> = this.behaviors.Invoke(\"CatalogService\", \"find\", [| BehaviorJSON.From(sku) |], fun () -> this.__tachyon_find(sku))"), "{program}");
        assert!(
            program.contains("type ProductConverter(behaviors: BehaviorContext) ="),
            "{program}"
        );
        assert!(program.contains("    member this.toSummary(product: BehaviorJSON) : BehaviorJSON = behaviors.MapValue(BehaviorJSON.Parse("), "{program}");
        assert!(
            detached
                .runtime
                .contains("if (type.Name == \"CatalogService\")")
        );
        assert!(detached.runtime.contains("public static System.Func<System.Type, BehaviorExecution, BehaviorAdapters> Configurations;"));
        assert!(!detached.runtime.contains("typeof(CatalogService)"));
        assert!(detached.setups.contains(
            "if (configuration == typeof(CatalogSetup)) return CatalogSetup.configure(execution);"
        ));
    }
}
