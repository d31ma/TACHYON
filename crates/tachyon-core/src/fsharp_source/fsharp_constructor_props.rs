use super::{FsharpType, FsharpView};
use std::path::Path;

pub(crate) struct FsharpConstructorProps {
    pub(crate) type_name: String,
    pub(crate) name: String,
}

impl FsharpConstructorProps {
    const NEEDS_ALL: &'static str = "NeedsAll";
    const ATTRIBUTE_OPEN: &'static str = "[<";

    pub(crate) fn needs_all(source: &str) -> bool {
        Self::has_needs_all_line(source)
    }

    pub(crate) fn takes_props(path: &Path, source: &str) -> bool {
        [
            super::FsharpSource::is_fsharp(path),
            Self::needs_all(source),
        ] == [true, true]
    }

    pub(crate) fn read(path: &Path, source: &str, class: &str) -> Vec<Self> {
        Self::from_shadow(FsharpView::tac(path, source).text(), class)
    }

    pub(crate) fn from_shadow(shadow: &str, class: &str) -> Vec<Self> {
        Self::written_parameters(shadow, class).map_or_else(Vec::new, Self::parameters)
    }

    fn has_needs_all_line(source: &str) -> bool {
        source.lines().any(Self::is_needs_all_line)
    }

    fn is_needs_all_line(line: &str) -> bool {
        let line = line.trim();
        [
            line.starts_with(Self::ATTRIBUTE_OPEN),
            line.contains(Self::NEEDS_ALL),
        ] == [true, true]
    }

    fn written_parameters<'a>(shadow: &'a str, class: &str) -> Option<&'a str> {
        let opening = format!("public {class}(");
        let start = shadow.find(&opening)? + opening.len();
        shadow[start..].split(')').next()
    }

    fn parameters(written: &str) -> Vec<Self> {
        FsharpType::split_top_level(written, ",")
            .into_iter()
            .filter_map(Self::parameter)
            .collect()
    }

    fn parameter(written: &str) -> Option<Self> {
        let (type_name, name) = written.trim().rsplit_once(char::is_whitespace)?;
        Some(Self {
            type_name: type_name.trim().to_owned(),
            name: name.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpConstructorProps;
    use std::path::Path;

    const SERVICE: &str = "[<Service>]\n[<NeedsAll>]\ntype SnippetService(files: string list, owner: string) =\n    member this.Owner : string = owner\n";

    #[test]
    fn the_primary_constructor_parameters_are_the_props() {
        let props = FsharpConstructorProps::read(
            Path::new("client/services/SnippetService.fs"),
            SERVICE,
            "SnippetService",
        );
        let written: Vec<(&str, &str)> = props
            .iter()
            .map(|prop| (prop.type_name.as_str(), prop.name.as_str()))
            .collect();
        assert_eq!(
            written,
            [("FSharpList<string>", "files"), ("string", "owner")]
        );
    }

    #[test]
    fn only_a_needs_all_type_takes_props() {
        assert!(FsharpConstructorProps::needs_all(SERVICE));
        assert!(FsharpConstructorProps::takes_props(
            Path::new("client/services/SnippetService.fs"),
            SERVICE
        ));
        assert!(!FsharpConstructorProps::takes_props(
            Path::new("client/services/SnippetService.cs"),
            SERVICE
        ));
        assert!(!FsharpConstructorProps::needs_all(
            "[<Service>]\ntype TallyService() =\n    member val Count : int = 0 with get, set\n"
        ));
        assert!(
            FsharpConstructorProps::from_shadow("public class TallyService {", "TallyService")
                .is_empty()
        );
    }
}
