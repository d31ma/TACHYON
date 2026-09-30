pub(crate) struct RustModelType;

impl RustModelType {
    const CONTAINERS: [&'static str; 2] = ["Vec", "Option"];
    const MODEL_SCALARS: [&'static str; 6] = ["String", "i64", "i32", "f64", "bool", "TacValue"];
    const NOT_MODELS: [&'static str; 2] = ["String", "TacValue"];
    const RUNTIME_PREFIX: &'static str = "Behavior";
    const JSON_SCALARS: [&'static str; 16] = [
        "TacValue", "String", "&str", "bool", "f32", "f64", "i8", "i16", "i32", "i64", "isize",
        "u8", "u16", "u32", "u64", "usize",
    ];

    pub(crate) fn is_model(kind: &str) -> bool {
        let kind = kind.trim();
        Self::container_item(kind).map_or_else(|| Self::is_named(kind), Self::is_model_item)
    }

    pub(crate) fn is_runtime(kind: &str) -> bool {
        kind.trim().starts_with(Self::RUNTIME_PREFIX)
    }

    pub(crate) fn crosses(kind: &str) -> bool {
        [Self::JSON_SCALARS.contains(&kind), Self::is_model(kind)].contains(&true)
    }

    pub(crate) fn read(expression: &str) -> String {
        format!("TacModel::write({expression})")
    }

    pub(crate) fn write(kind: &str) -> String {
        format!("<{} as TacModel>::read(&value)", kind.trim())
    }

    fn is_model_item(item: &str) -> bool {
        Self::MODEL_SCALARS.contains(&item) || Self::is_model(item)
    }

    fn is_named(kind: &str) -> bool {
        let checks = [
            kind.starts_with(|character: char| character.is_ascii_uppercase()),
            kind.chars().all(Self::is_name_character),
            !Self::NOT_MODELS.contains(&kind),
            !Self::is_runtime(kind),
            !crate::web_api_clients::WebApiClients::is_client_type(kind),
        ];
        checks == [true; 5]
    }

    fn is_name_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_'
    }

    fn container_item(kind: &str) -> Option<&str> {
        let (base, rest) = kind.split_once('<')?;
        let item = rest.strip_suffix('>')?.trim();
        Self::CONTAINERS.contains(&base.trim()).then_some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::RustModelType;

    #[test]
    fn a_model_a_list_of_models_and_an_optional_model_are_models() {
        for kind in [
            "Product",
            "Vec<Product>",
            "Option<Product>",
            "Vec<String>",
            "Vec<Vec<Maker>>",
        ] {
            assert!(RustModelType::is_model(kind), "{kind}");
        }
    }

    #[test]
    fn scalars_paths_and_other_collections_are_not_models() {
        for kind in [
            "String",
            "TacValue",
            "i32",
            "BehaviorContext",
            "std::path::PathBuf",
            "Vec<u8>",
            "HashMap<String, i32>",
        ] {
            assert!(!RustModelType::is_model(kind), "{kind}");
        }
    }

    #[test]
    fn behavior_runtime_types_are_not_models_and_do_not_cross() {
        for kind in [
            "BehaviorValue",
            "BehaviorContext",
            "BehaviorError",
            "Vec<BehaviorValue>",
            "Option<BehaviorValue>",
            "BehaviorResult<BehaviorValue>",
        ] {
            assert!(!RustModelType::is_model(kind), "{kind}");
            assert!(!RustModelType::crosses(kind), "{kind}");
        }
        assert!(RustModelType::is_runtime("BehaviorValue"));
        assert!(!RustModelType::is_runtime("Product"));
    }

    #[test]
    fn json_scalars_and_models_cross_to_the_page() {
        for kind in ["&str", "u8", "Product", "Vec<Product>"] {
            assert!(RustModelType::crosses(kind), "{kind}");
        }
        assert!(!RustModelType::crosses("std::path::PathBuf"));
    }

    #[test]
    fn a_model_crosses_through_the_tac_model_trait() {
        assert_eq!(
            RustModelType::read("it.borrow().product.clone()"),
            "TacModel::write(it.borrow().product.clone())"
        );
        assert_eq!(
            RustModelType::write("Vec<Product>"),
            "<Vec<Product> as TacModel>::read(&value)"
        );
    }
}
