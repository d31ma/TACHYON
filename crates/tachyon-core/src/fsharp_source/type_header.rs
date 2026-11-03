use super::{FsharpParameters, FsharpType, RecordFields, TypeForm, TypeName};

pub(crate) struct TypeHeader {
    pub(crate) name: String,
    pub(crate) form: TypeForm,
    pub(crate) lines: Vec<String>,
    pub(crate) parameters: String,
}

impl TypeHeader {
    const KEYWORDS: [&'static str; 2] = ["type ", "and "];
    const ACCESS: [(&'static str, &'static str); 3] = [
        ("private ", "internal "),
        ("internal ", "internal "),
        ("public ", "public "),
    ];

    pub(crate) fn read(trimmed: &str, body: &[&str], managed: bool) -> Self {
        let declared = Self::declared_name(trimmed);
        let (parameters, after_parameters) = FsharpParameters::read(declared.rest);
        let after_equals = after_parameters.trim_start().trim_start_matches('=').trim();
        let has_constructor = declared.rest.trim_start().starts_with('(');
        let form = Self::form(has_constructor, after_equals, body);
        let mut lines = Self::needs_line(&parameters, managed);
        lines.push(Self::declaration(&declared, form, &parameters, body));
        lines.push(Self::single_line_body(form, after_equals));
        lines.retain(|line| !line.is_empty());
        Self {
            name: declared.name.to_owned(),
            form,
            lines,
            parameters,
        }
    }

    pub(crate) fn parameter_type(parameters: &str, name: &str) -> Option<String> {
        FsharpType::split_top_level(parameters, ",")
            .into_iter()
            .filter_map(|parameter| parameter.rsplit_once(' '))
            .find(|(_, named)| *named == name)
            .map(|(kind, _)| Self::without_attributes(kind))
    }

    pub(crate) fn constructor_types(trimmed: &str) -> Vec<String> {
        let declared = Self::declared_name(trimmed);
        Self::parameter_types(&FsharpParameters::read(declared.rest).0)
    }

    pub(crate) fn exception(trimmed: &str) -> Self {
        let declared = trimmed.trim_start_matches("exception").trim();
        let name = declared.split_whitespace().next().unwrap_or(declared);
        Self {
            name: name.to_owned(),
            form: TypeForm::Class,
            lines: vec![format!("public class {name} : System.Exception {{")],
            parameters: String::new(),
        }
    }

    fn declared_name(trimmed: &str) -> TypeName<'_> {
        let after_keyword = Self::after_keyword(trimmed);
        let access = Self::access(after_keyword);
        let rest = Self::without_access_word(after_keyword);
        let end = rest
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(rest.len());
        let (generics, after) = Self::generics(&rest[end..]);
        TypeName {
            access,
            name: &rest[..end],
            generics,
            rest: after,
        }
    }

    fn after_keyword(trimmed: &str) -> &str {
        Self::KEYWORDS
            .iter()
            .find_map(|keyword| trimmed.strip_prefix(keyword))
            .unwrap_or(trimmed)
            .trim_start()
    }

    fn access(rest: &str) -> &'static str {
        Self::ACCESS
            .iter()
            .find(|(fsharp, _)| rest.starts_with(fsharp))
            .map_or("public ", |(_, csharp)| *csharp)
    }

    fn without_access_word(rest: &str) -> &str {
        Self::ACCESS
            .iter()
            .find_map(|(fsharp, _)| rest.strip_prefix(fsharp))
            .unwrap_or(rest)
            .trim_start()
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    fn generics(after_name: &str) -> (String, &str) {
        let Some(inside) = after_name.strip_prefix('<') else {
            return (String::new(), after_name);
        };
        let (list, rest) = inside.split_once('>').unwrap_or((inside, ""));
        (format!("<{}>", list.replace('\'', "")), rest)
    }

    fn form(has_constructor: bool, after_equals: &str, body: &[&str]) -> TypeForm {
        if has_constructor {
            return TypeForm::Class;
        }
        TypeForm::of(after_equals, body)
    }

    fn needs_line(parameters: &str, managed: bool) -> Vec<String> {
        let types = Self::parameter_types(parameters);
        let wanted = managed && !types.is_empty();
        wanted
            .then(|| format!("[Needs({})]", types.join(", ")))
            .into_iter()
            .collect()
    }

    fn parameter_types(parameters: &str) -> Vec<String> {
        FsharpType::split_top_level(parameters, ",")
            .into_iter()
            .filter(|parameter| !parameter.is_empty())
            .filter_map(|parameter| parameter.rsplit_once(' '))
            .map(|(kind, _)| Self::without_attributes(kind))
            .collect()
    }

    fn without_attributes(kind: &str) -> String {
        kind.rsplit(']').next().unwrap_or(kind).trim().to_owned()
    }

    fn declaration(
        declared: &TypeName<'_>,
        form: TypeForm,
        parameters: &str,
        body: &[&str],
    ) -> String {
        let TypeName {
            access,
            name,
            generics,
            ..
        } = declared;
        let bases = Self::bases(body);
        match form {
            TypeForm::Record | TypeForm::Class | TypeForm::Union => format!(
                "{access}class {name}{generics}{bases} {{{}",
                Self::constructor(name, parameters)
            ),
            TypeForm::Enum => format!("{access}enum {name} {{"),
            TypeForm::Interface => format!("{access}interface {name}{generics} {{"),
            TypeForm::Alias => String::new(),
        }
    }

    fn constructor(name: &str, parameters: &str) -> String {
        Some(parameters)
            .filter(|parameters| !parameters.is_empty())
            .map_or_else(String::new, |parameters| {
                format!(" public {name}({parameters}) {{ }}")
            })
    }

    fn bases(body: &[&str]) -> String {
        let mut bases = Vec::new();
        for line in body {
            bases.extend(Self::base(line));
        }
        let listed = (!bases.is_empty()).then(|| format!(" : {}", bases.join(", ")));
        listed.unwrap_or_default()
    }

    fn base(line: &str) -> Option<String> {
        let inherited = line.strip_prefix("inherit ");
        let implemented = line
            .strip_prefix("interface ")
            .and_then(|rest| rest.strip_suffix(" with"));
        let written = inherited.or(implemented)?;
        let name = written.split('(').next().unwrap_or(written).trim();
        Some(FsharpType::csharp(name))
    }

    fn single_line_body(form: TypeForm, after_equals: &str) -> String {
        let cases = FsharpType::split_top_level(after_equals.trim_start_matches('|'), "|");
        match form {
            TypeForm::Enum => cases.join(", "),
            TypeForm::Record => RecordFields::csharp(after_equals),
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TypeForm, TypeHeader};

    #[test]
    fn a_managed_class_lists_its_constructor_types_as_needs() {
        let header = TypeHeader::read(
            "type OrderService(store: OrderStore, clock: Clock) =",
            &[],
            true,
        );
        assert_eq!(header.name, "OrderService");
        assert_eq!(header.form, TypeForm::Class);
        assert_eq!(
            header.lines,
            vec![
                String::from("[Needs(OrderStore, Clock)]"),
                String::from(
                    "public class OrderService { public OrderService(OrderStore store, Clock clock) { }"
                ),
            ]
        );
    }

    #[test]
    fn records_enums_interfaces_and_bases() {
        let record = TypeHeader::read("type Product =", &["{ Name: string"], false);
        assert_eq!(record.lines, vec![String::from("public class Product {")]);
        let status = TypeHeader::read("type Status = Active | Closed", &[], false);
        assert_eq!(
            status.lines,
            vec![
                String::from("public enum Status {"),
                String::from("Active, Closed")
            ]
        );
        let store = TypeHeader::read(
            "type IStore =",
            &["abstract member Find : int -> Order"],
            false,
        );
        assert_eq!(store.lines, vec![String::from("public interface IStore {")]);
        let failure = TypeHeader::read(
            "type OrderMissing(message: string) =",
            &["inherit Exception(message)"],
            false,
        );
        assert_eq!(
            failure.lines,
            vec![String::from(
                "public class OrderMissing : Exception { public OrderMissing(string message) { }"
            )]
        );
    }

    #[test]
    fn private_generic_and_alias_types() {
        let boxed = TypeHeader::read("type private Box<'T>() =", &[], false);
        assert_eq!(boxed.lines, vec![String::from("internal class Box<T> {")]);
        let product = TypeHeader::read(
            "type Product = { Name: string; mutable Price: float }",
            &[],
            false,
        );
        assert_eq!(
            product.lines,
            vec![
                String::from("public class Product {"),
                String::from("public string Name { get; init; } public double Price { get; set; }")
            ]
        );
        let alias = TypeHeader::read("type Id = int", &[], false);
        assert!(alias.lines.is_empty());
        let failure = TypeHeader::exception("exception OrderMissing of string");
        assert_eq!(failure.name, "OrderMissing");
    }
}
