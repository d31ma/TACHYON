pub(crate) struct CsharpModelType;

impl CsharpModelType {
    const SCALARS: [&'static str; 11] = [
        "string",
        "int",
        "long",
        "double",
        "float",
        "bool",
        "decimal",
        "object",
        "void",
        "Task",
        "BehaviorJSON",
    ];
    const LISTS: [&'static str; 6] = [
        "List",
        "IList",
        "IEnumerable",
        "IReadOnlyList",
        "ICollection",
        "IReadOnlyCollection",
    ];
    const FSHARP_LIST: &'static str = "FSharpList";
    const TASKS: [&'static str; 2] = ["Task", "ValueTask"];
    const ARRAY: &'static str = "[]";
    const MODELS_NAMESPACE: &'static str = "Models.";
    const RUNTIME_PREFIX: &'static str = "Behavior";
    const BEHAVIOR_JSON: &'static str = "BehaviorJSON";
    const TYPE_SEPARATORS: [char; 6] = ['<', '>', ',', '[', ']', ' '];

    pub(crate) fn is_model(kind: &str) -> bool {
        let kind = kind.trim();
        let checks = [
            !Self::is_foreign(kind),
            !Self::SCALARS.contains(&kind),
            !Self::names_runtime(kind),
            !crate::web_api_clients::WebApiClients::is_client_type(kind),
        ];
        checks == [true; 4]
    }

    pub(crate) fn is_runtime(kind: &str) -> bool {
        let kind = kind.trim();
        [
            kind.starts_with(Self::RUNTIME_PREFIX),
            kind != Self::BEHAVIOR_JSON,
        ] == [true, true]
    }

    pub(crate) fn is_task(kind: &str) -> bool {
        Self::generic(kind).is_some_and(|(base, _)| Self::TASKS.contains(&base))
    }

    pub(crate) fn plain(expression: &str) -> String {
        format!("TacModels.Plain({expression})")
    }

    pub(crate) fn value(kind: Option<&str>, expression: &str) -> String {
        Self::chosen_value(kind.is_some_and(Self::is_model), expression)
    }

    pub(crate) fn reading(kind: &str, value: &str) -> Option<String> {
        Self::chosen_reading(Self::is_model(kind), kind, value)
    }

    pub(crate) fn read(kind: &str, value: &str) -> Option<String> {
        let kind = kind.trim();
        kind.strip_suffix(Self::ARRAY).map_or_else(
            || Self::read_named(kind, value),
            |item| Some(format!("{}.ToArray()", Self::read_list(item, value))),
        )
    }

    pub(crate) fn declared(line: &str, name_end: usize) -> Option<(String, &str)> {
        let generic_end = name_end + Self::generic_length(&line[name_end..])?;
        let end = generic_end + Self::array_length(&line[generic_end..]);
        Some((line[..end].to_owned(), line[end..].trim_start()))
    }

    fn chosen_value(model: bool, expression: &str) -> String {
        if model {
            return Self::plain(expression);
        }
        expression.to_owned()
    }

    fn chosen_reading(model: bool, kind: &str, value: &str) -> Option<String> {
        if model {
            return Self::read(kind, value);
        }
        Some(super::csharp_converter(kind).replace("value", value))
    }

    fn names_runtime(kind: &str) -> bool {
        kind.split(Self::TYPE_SEPARATORS)
            .any(|part| part.starts_with(Self::RUNTIME_PREFIX))
    }

    fn is_foreign(kind: &str) -> bool {
        let base = kind.split('<').next().unwrap_or(kind);
        [
            base.contains('.'),
            !base.starts_with(Self::MODELS_NAMESPACE),
        ] == [true, true]
    }

    fn read_named(kind: &str, value: &str) -> Option<String> {
        Self::generic(kind).map_or_else(
            || Some(format!("TacModels.Read<{kind}>({value})")),
            |(base, item)| Self::read_generic(base, item, value),
        )
    }

    fn read_generic(base: &str, item: &str, value: &str) -> Option<String> {
        let list = Self::read_list(item, value);
        let fsharp = format!("Microsoft.FSharp.Collections.ListModule.OfSeq({list})");
        let readings = [
            (Self::LISTS.contains(&base), list),
            (base == Self::FSHARP_LIST, fsharp),
        ];
        readings
            .into_iter()
            .find_map(|(matches, reading)| matches.then_some(reading))
    }

    fn read_list(item: &str, value: &str) -> String {
        format!("TacModels.ReadList<{}>({value})", item.trim())
    }

    fn generic_length(text: &str) -> Option<usize> {
        let opens = text.starts_with('<');
        opens
            .then(|| Self::closing_angle(text))
            .map_or(Some(0), |closing| closing.and_then(|at| at.checked_add(1)))
    }

    fn array_length(text: &str) -> usize {
        usize::from(text.starts_with(Self::ARRAY)) * Self::ARRAY.len()
    }

    fn closing_angle(text: &str) -> Option<usize> {
        let mut depth = 0_i32;
        text.char_indices().find_map(|(at, character)| {
            depth += Self::depth_change(character);
            (depth == 0).then_some(at)
        })
    }

    fn depth_change(character: char) -> i32 {
        match character {
            '<' => 1,
            '>' => -1,
            _ => 0,
        }
    }

    fn generic(kind: &str) -> Option<(&str, &str)> {
        let (base, rest) = kind.split_once('<')?;
        let item = rest.strip_suffix('>')?;
        Some((base.trim(), item.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::CsharpModelType;

    #[test]
    fn records_and_their_lists_are_models_and_scalars_are_not() {
        for kind in [
            "Product",
            "List<Product>",
            "Product[]",
            "FSharpList<Product>",
        ] {
            assert!(CsharpModelType::is_model(kind), "{kind}");
        }
        assert!(CsharpModelType::is_model("Models.Product"));
        for kind in [
            "string",
            "int",
            "object",
            "BehaviorJSON",
            "BehaviorContext",
            "System.IO.Stream",
        ] {
            assert!(!CsharpModelType::is_model(kind), "{kind}");
        }
    }

    #[test]
    fn behavior_runtime_types_and_their_lists_are_not_models() {
        for kind in [
            "BehaviorSignal",
            "BehaviorExecution",
            "List<BehaviorJSON>",
            "BehaviorJSON[]",
            "Dictionary<string, BehaviorSignal>",
        ] {
            assert!(!CsharpModelType::is_model(kind), "{kind}");
        }
        assert!(CsharpModelType::is_runtime("BehaviorSignal"));
        assert!(!CsharpModelType::is_runtime("BehaviorJSON"));
        assert!(!CsharpModelType::is_runtime("Product"));
    }

    #[test]
    fn every_list_kind_is_read_item_by_item() {
        let value = "value";
        let read = |kind: &str| CsharpModelType::read(kind, value).unwrap_or_default();
        assert_eq!(read("Product"), "TacModels.Read<Product>(value)");
        assert_eq!(read("List<Product>"), "TacModels.ReadList<Product>(value)");
        assert_eq!(
            read("Product[]"),
            "TacModels.ReadList<Product>(value).ToArray()"
        );
        assert_eq!(
            read("FSharpList<Product>"),
            "Microsoft.FSharp.Collections.ListModule.OfSeq(TacModels.ReadList<Product>(value))"
        );
        assert_eq!(
            CsharpModelType::read("Dictionary<string, int>", value),
            None
        );
    }

    #[test]
    fn a_generic_or_array_type_is_read_whole() {
        fn declared(line: &str) -> Option<(String, &str)> {
            CsharpModelType::declared(line, line.find(['<', '[', ' ']).unwrap_or(0))
        }
        assert_eq!(
            declared("List<List<Product>> Shelves;"),
            Some((String::from("List<List<Product>>"), "Shelves;"))
        );
        assert_eq!(
            declared("Product[] Items;"),
            Some((String::from("Product[]"), "Items;"))
        );
        assert_eq!(
            declared("string Name;"),
            Some((String::from("string"), "Name;"))
        );
        assert_eq!(declared("List<Product Broken"), None);
    }

    #[test]
    fn a_model_value_crosses_through_the_model_context() {
        assert_eq!(
            CsharpModelType::value(Some("Product"), "Instance.Product"),
            "TacModels.Plain(Instance.Product)"
        );
        assert_eq!(
            CsharpModelType::value(Some("int"), "Instance.Count"),
            "Instance.Count"
        );
        assert_eq!(
            CsharpModelType::reading("int", "given").as_deref(),
            Some("Convert.ToInt32(given)")
        );
    }

    #[test]
    fn a_task_of_a_value_is_a_task() {
        assert!(CsharpModelType::is_task("Task<int>"));
        assert!(!CsharpModelType::is_task("List<int>"));
    }
}
