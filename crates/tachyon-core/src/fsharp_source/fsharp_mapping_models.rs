use super::mapping_record::MappingRecord;
use crate::route_types::{Model, ModelTable};
use std::path::PathBuf;

pub(crate) struct FsharpMappingModels;

impl FsharpMappingModels {
    pub(crate) fn program(program: &str, sources: &[(PathBuf, String)]) -> Result<String, String> {
        Self::mapping_program(program).map_or_else(
            || Ok(program.to_owned()),
            |program| Self::generated(program, sources),
        )
    }

    fn mapping_program(program: &str) -> Option<&str> {
        program.contains("TacMappingModels.").then_some(program)
    }

    fn generated(program: &str, sources: &[(PathBuf, String)]) -> Result<String, String> {
        let models = Self::models(sources)?;
        let parts = program.split("namespace rec ").collect::<Vec<_>>();
        let namespaces = parts
            .iter()
            .skip(1)
            .map(|part| format!("namespace rec {part}\n{models}"))
            .collect::<Vec<_>>()
            .concat();
        Ok(format!("{}{namespaces}", parts[0]))
    }

    fn models(sources: &[(PathBuf, String)]) -> Result<String, String> {
        let records = sources
            .iter()
            .flat_map(|(_, source)| Self::records(source))
            .collect::<Vec<_>>();
        let functions = records
            .iter()
            .map(|(name, source)| MappingRecord::functions(name, source))
            .collect::<Result<Vec<_>, _>>()?
            .concat()
            .join("\n    and ");
        let cases = records
            .iter()
            .map(|(name, _)| format!("        | :? Models.{name} as model -> write{name} model\n"))
            .collect::<Vec<_>>()
            .concat();
        Ok(format!(
            "module TacMappingModels =\n    let rec {functions}\n    let write (value: obj) =\n        match value with\n{cases}        | _ -> raise (BehaviorError(\"JSON_TYPE\"))\n"
        ))
    }

    fn records(source: &str) -> Vec<(String, &str)> {
        ModelTable::declared_models("fs", source)
            .into_iter()
            .filter_map(|entry| Self::record(entry, source))
            .collect()
    }

    fn record((name, model): (String, Model), source: &str) -> Option<(String, &str)> {
        match model {
            Model::Record(_) => Some((name, source)),
            Model::Alias(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpMappingModels;
    use std::path::PathBuf;

    #[test]
    fn nested_lists_and_options_use_static_record_codecs() {
        let sources = [(
            PathBuf::from("Models.fs"),
            String::from(
                "type Detail = { name: string }\ntype Product = { count: int; id: int64; array: Detail array; detail: Detail option; items: Detail list }",
            ),
        )];
        let output = FsharpMappingModels::program(
            "namespace rec Test\nTacMappingModels.readProduct value",
            &sources,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert!(output.contains("Checked.int ((value.Get(\"count\")).Integer)"));
        assert!(output.contains(
            "((value.Get(\"array\")).Items |> Array.map (fun item -> readDetail (item)))"
        ));
        assert!(output.contains("Checked.int64 ((value.Get(\"id\")).Integer)"));
        assert!(output.contains("None else Some(readDetail"));
        assert!(output.contains("Array.map (fun item -> readDetail (item)) |> Array.toList"));
        assert!(output.contains("Some item -> writeDetail item"));
        assert!(output.contains("BehaviorJSON.Array(value.items"));
        assert!(!output.contains("JsonSerializer"));
    }

    #[test]
    fn unrelated_services_do_not_generate_codecs() {
        assert_eq!(
            FsharpMappingModels::program("namespace rec Test\ntype Service() = class end", &[])
                .unwrap_or_else(|error| panic!("{error}")),
            "namespace rec Test\ntype Service() = class end"
        );
    }
}
