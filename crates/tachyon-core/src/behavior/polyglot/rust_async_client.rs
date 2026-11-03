use super::{Method, Parameter, Type};

pub(super) struct RustAsyncClient;

impl RustAsyncClient {
    const RESULT_START: &'static str = "BehaviorResult<";
    const RESULT_END: &'static str = ">";
    const RAW_ANSWER: &'static str = "BehaviorValue";
    const NO_ANSWER: &'static str = "()";
    const OBJECT_READER: &'static str = "object";
    const READERS: [(&'static str, &'static str); 7] = [
        ("Vec<", "list"),
        ("Option<", "optional"),
        ("String", "plain"),
        ("i64", "plain"),
        ("i32", "plain"),
        ("f64", "plain"),
        ("bool", "plain"),
    ];
    const PARAMETERS: [&'static str; 5] = ["BehaviorValue", "String", "&str", "bool", "i32"];
    const REFUSED_POLICIES: [&'static str; 3] = ["retryable", "concurrencyLimit", "transactional"];
    const EXCHANGE: &'static str = "httpExchange";
    const GENERATED: &'static str = "generated";
    const OPEN: char = '<';
    const PLAIN_ENTRY: &'static str = "plain";
    const ENTRIES: [(&'static str, &'static str); 3] = [
        ("cacheable", "cached"),
        ("cacheEvict", "mutated"),
        ("cachePut", "mutated"),
    ];

    pub(super) fn applies(method: &Method) -> bool {
        [method.asynchronous, Self::generated(method)] == [true, true]
    }

    pub(super) fn reads_models(declaration: &Type) -> bool {
        declaration
            .methods
            .iter()
            .filter(|method| Self::applies(method))
            .filter_map(Self::answer)
            .any(|answer| Self::converts(&answer))
    }

    pub(super) fn validate(method: &Method) -> Result<(), String> {
        Self::require_receiver(method)?;
        Self::require_answer(method)?;
        Self::require_parameters(method)?;
        Self::require_policies(method)
    }

    pub(super) fn body(method: &Method, policy: &str) -> String {
        let arguments = method
            .parameters
            .iter()
            .map(|parameter| format!("BehaviorValue::from({})", parameter.name))
            .collect::<Vec<_>>()
            .join(", ");
        let exchange = format!(
            "BehaviorExchange::{}(&self.behaviors, {policy}, vec![{arguments}]).await",
            Self::entry(method)
        );
        let answer = Self::answer(method).unwrap_or_else(|| Self::RAW_ANSWER.to_owned());
        Self::finished(&answer, &exchange)
    }

    fn entry(method: &Method) -> &'static str {
        Self::ENTRIES
            .iter()
            .find(|(key, _)| method.metadata.contains_key(*key))
            .map_or(Self::PLAIN_ENTRY, |(_, entry)| *entry)
    }

    fn generated(method: &Method) -> bool {
        method
            .metadata
            .get(Self::EXCHANGE)
            .and_then(|exchange| exchange.get(Self::GENERATED))
            .is_some()
    }

    fn converts(answer: &str) -> bool {
        ![Self::RAW_ANSWER, Self::NO_ANSWER].contains(&answer)
    }

    fn require_receiver(method: &Method) -> Result<(), String> {
        if method.is_static {
            return Err(format!(
                "Rust HTTPClient method {} takes &self; a static method cannot use the client's context",
                method.name
            ));
        }
        Ok(())
    }

    fn require_answer(method: &Method) -> Result<(), String> {
        Self::answer(method).map(|_| ()).ok_or_else(|| {
            format!(
                "Async Rust HTTPClient method {} returns BehaviorResult<T>: T is BehaviorValue, a model from client/models, a list of models or a plain value",
                method.name
            )
        })
    }

    fn require_parameters(method: &Method) -> Result<(), String> {
        method
            .parameters
            .iter()
            .find(|parameter| Self::refused(parameter))
            .map_or(Ok(()), |parameter| {
                Err(format!(
                    "Async Rust HTTPClient method {} takes {} as {}; use BehaviorValue, String, &str, bool or i32",
                    method.name, parameter.name, parameter.type_source
                ))
            })
    }

    fn require_policies(method: &Method) -> Result<(), String> {
        Self::REFUSED_POLICIES
            .iter()
            .find(|policy| method.metadata.contains_key(**policy))
            .map_or(Ok(()), |policy| {
                Err(format!(
                    "Async Rust HTTPClient method {} cannot use {policy} yet; the browser request is already bounded by its deadline",
                    method.name
                ))
            })
    }

    fn refused(parameter: &Parameter) -> bool {
        !Self::PARAMETERS.contains(&parameter.type_source.trim())
    }

    fn finished(answer: &str, exchange: &str) -> String {
        match answer {
            Self::NO_ANSWER => format!("\n{exchange}?;\nOk(())\n"),
            Self::RAW_ANSWER => format!("\n{exchange}\n"),
            _ => format!(
                "\nlet __tachyon_answer = {exchange}?;\nBehaviorTypedAnswer::{}::<{answer}>(__tachyon_answer)\n",
                Self::reader(answer)
            ),
        }
    }

    fn reader(answer: &str) -> &'static str {
        Self::READERS
            .iter()
            .find(|(pattern, _)| Self::fits(answer, pattern))
            .map_or(Self::OBJECT_READER, |(_, reader)| *reader)
    }

    fn fits(answer: &str, pattern: &str) -> bool {
        let opens = [pattern.ends_with(Self::OPEN), answer.starts_with(pattern)] == [true, true];
        [opens, answer == pattern].contains(&true)
    }

    fn answer(method: &Method) -> Option<String> {
        let compact = method
            .return_type
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let inner = compact
            .strip_prefix(Self::RESULT_START)?
            .strip_suffix(Self::RESULT_END)?;
        (!inner.is_empty()).then(|| inner.to_owned())
    }
}
