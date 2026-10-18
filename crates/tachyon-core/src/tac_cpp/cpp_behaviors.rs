use super::{CppAttribute, CppClass, CppMember, CppMemberKind};
use crate::Failure;
use crate::behavior::{WrittenMarker, WrittenPolicies};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct CppBehaviors;

type CppMarked<'m> = (&'m [&'m CppAttribute], &'m [(usize, &'m CppAttribute)]);

impl CppBehaviors {
    const CACHE: &'static str = "CacheSettings";
    const PARTITION: &'static str = "default";
    const SHAPES: [&'static str; 3] = [
        "A Yon C++ method with [[Cached]], [[UpdateCache]], [[ClearCache]], [[Retry]], [[MaxAtOnce]], [[AllOrNothing]] or a [[PageDefaults]] parameter is a synchronous instance method with a body that returns a value or void, not a reference: Tachyon wraps the body",
        "A C++ method with [[Cached]], [[UpdateCache]], [[ClearCache]], [[Retry]], [[MaxAtOnce]], [[AllOrNothing]] or a [[PageDefaults]] parameter is an instance method with a body that returns a value, void or a Task, not a reference: Tachyon wraps the body. [[AllOrNothing]] goes on a method that returns no Task, because its transaction cannot stay open while the method waits",
        "A C++ [[HTTPClient]] method takes only [[Cached]], [[UpdateCache]] or [[ClearCache]] and [[CacheKey]] parameters next to its request marker; [[Retry]], [[MaxAtOnce]], [[AllOrNothing]] and [[PageDefaults]] belong on a service method",
    ];
    const OPENINGS: [&'static str; 2] = [
        " static const ::tachyon::json::JsonNode tachyonPolicy = ::tachyon::json::JsonNode::parse({literal}); return ::tachyon::TacBehaviors::run<{result}>(tachyonPolicy, [&]() { return std::vector<::tachyon::json::JsonNode>{{encoded}}; }, [&](const std::vector<::tachyon::json::JsonNode>& tachyonArguments) -> {result} { (void)tachyonArguments;{page}",
        " static const ::tachyon::json::JsonNode tachyonPolicy = ::tachyon::json::JsonNode::parse({literal}); co_return co_await ::tachyon::TacTaskBehaviors::run<{answer}>(tachyonPolicy, std::vector<::tachyon::json::JsonNode>{{encoded}}, nullptr, [&](const std::vector<::tachyon::json::JsonNode>& tachyonArguments, ::tachyon::TacCall) -> {result} { (void)tachyonArguments;{page}",
    ];
    const CLIENT_MARKERS: [&'static str; 3] = ["Cached", "ClearCache", "UpdateCache"];
    const TRANSACTION: &'static str = "AllOrNothing";
    const NEEDS_CACHE: &'static str = "[[Cached]], [[UpdateCache]] and [[ClearCache]] need [[CacheSettings]] on their C++ class, for example `class [[Service, CacheSettings({\"name\":\"prices\",\"ttlMs\":60000,\"maxBytes\":65536,\"maxEntries\":100})]] PriceService`";
    const METHOD_MARKERS: [&'static str; 6] = [
        "Cached",
        "ClearCache",
        "UpdateCache",
        "Retry",
        "MaxAtOnce",
        "AllOrNothing",
    ];
    const NEEDS_CACHED: &'static str =
        "[[CacheKey]] marks a key parameter of a [[Cached]] method; add [[Cached]] to the method";
    const RETURNS: &'static str = "A C++ [[Cached]] or [[UpdateCache]] method returns the value that Tachyon keeps, so it cannot return void";
    const TWICE: &'static str = "Each behavior marker appears once on a C++ method";
    const KEY_FORM: &'static str =
        "[[CacheKey]] takes no arguments: it marks one key parameter of a [[Cached]] method";
    const BARE_PAGE: &'static str = "[[PageDefaults]] without sizes marks the page parameter of a Store query; on other methods write the sizes: [[PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]})]]";

    pub(crate) fn policies(
        path: &Path,
        source: &str,
        class: &CppClass,
    ) -> Result<BTreeMap<String, Value>, Failure> {
        let written = WrittenPolicies::new(path, source, super::CppSource::EXTENSION);
        let shape = Self::shape(path, class);
        let cache = class
            .markers
            .iter()
            .find(|marker| marker.name == Self::CACHE)
            .map(|marker| written.cache(&Self::written(marker)))
            .transpose()?;
        let found = class
            .members
            .iter()
            .map(|member| {
                Self::metadata(&written, source, member, shape)
                    .map(|metadata| metadata.map(|metadata| (member.name.clone(), metadata)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut methods: BTreeMap<String, Map<String, Value>> =
            found.into_iter().flatten().collect();
        written.resolve_mutations(class.name_at, &mut methods)?;
        let module = path.to_string_lossy().replace('\\', "/");
        methods
            .iter()
            .map(|(name, metadata)| {
                Self::policy(&written, class, name, metadata, cache.as_ref(), &module)
                    .map(|policy| (name.clone(), policy))
            })
            .collect()
    }

    pub(crate) fn wrapping(member: &CppMember, policy: &Value) -> (String, String) {
        let result = member.plain_type();
        let literal = Value::String(policy.to_string()).to_string();
        let encoded: String = Self::needs_arguments(policy)
            .then(|| Self::encoded(member))
            .into_iter()
            .collect();
        let page = Self::page_parameter(member, policy);
        let opening = Self::OPENINGS[usize::from(member.is_task())]
            .replace("{literal}", &literal)
            .replace("{answer}", &super::CppRemote::answer_type(member))
            .replace("{result}", &result)
            .replace("{encoded}", &encoded)
            .replace("{page}", &page);
        (opening, String::from(" }); "))
    }

    pub(crate) fn encoded(member: &CppMember) -> String {
        member
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "::tachyon::json::JsonCodec<std::decay_t<decltype({0})>>::toJson({0})",
                    parameter.name
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn needs_arguments(policy: &Value) -> bool {
        ["keyArgs", "mutation", "pageable"]
            .iter()
            .any(|key| !policy[*key].is_null())
    }

    fn page_parameter(member: &CppMember, policy: &Value) -> String {
        let index = policy["pageable"]["argument"]
            .as_u64()
            .and_then(|index| usize::try_from(index).ok());
        let Some(parameter) = index.and_then(|index| member.parameters.get(index)) else {
            return String::new();
        };
        let name = &parameter.name;
        format!(
            " using TachyonPage = std::decay_t<decltype({name})>; const TachyonPage {name} = ::tachyon::json::JsonCodec<TachyonPage>::fromJson(tachyonArguments[{}]);",
            index.unwrap_or_default()
        )
    }

    fn written(marker: &CppAttribute) -> WrittenMarker<'_> {
        WrittenMarker {
            name: &marker.name,
            arguments: &marker.arguments,
            at: marker.at,
        }
    }

    fn shape(path: &Path, class: &CppClass) -> usize {
        let tac =
            crate::annotation_rules::Realm::of(path) != Some(crate::annotation_rules::Realm::Yon);
        let client = class
            .markers
            .iter()
            .any(|marker| marker.name == "HTTPClient");
        usize::from(tac) + usize::from([tac, client] == [true, true])
    }

    fn metadata(
        written: &WrittenPolicies<'_>,
        source: &str,
        member: &CppMember,
        shape: usize,
    ) -> Result<Option<Map<String, Value>>, Failure> {
        let method_markers: Vec<&CppAttribute> = member
            .markers
            .iter()
            .filter(|marker| Self::METHOD_MARKERS.contains(&marker.name.as_str()))
            .collect();
        let parameter_markers: Vec<(usize, &CppAttribute)> = member
            .parameters
            .iter()
            .enumerate()
            .flat_map(|(index, parameter)| {
                parameter
                    .markers
                    .iter()
                    .filter(|marker| ["CacheKey", "PageDefaults"].contains(&marker.name.as_str()))
                    .map(move |marker| (index, marker))
            })
            .collect();
        let marked = [method_markers.is_empty(), parameter_markers.is_empty()] != [true, true];
        marked
            .then(|| {
                Self::marked_metadata(
                    written,
                    source,
                    member,
                    (&method_markers, &parameter_markers),
                    shape,
                )
            })
            .transpose()
    }

    fn marked_metadata(
        written: &WrittenPolicies<'_>,
        source: &str,
        member: &CppMember,
        (method_markers, parameter_markers): (&[&CppAttribute], &[(usize, &CppAttribute)]),
        shape: usize,
    ) -> Result<Map<String, Value>, Failure> {
        let shapes = [Self::yon_shape, Self::service_shape, Self::client_shape];
        shapes[shape](source, member, (method_markers, parameter_markers))
            .then_some(())
            .ok_or_else(|| written.failure(member.name_at, Self::SHAPES[shape]))?;
        let mut metadata = Map::new();
        method_markers.iter().try_for_each(|marker| {
            Self::add_method_marker(written, &mut metadata, member, marker)
        })?;
        let keys: Vec<Value> = parameter_markers
            .iter()
            .map(|(index, marker)| {
                Self::add_parameter_marker(written, &mut metadata, *index, marker)
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Self::add_keys(written, &mut metadata, member, keys)?;
        let names: Vec<Value> = member
            .parameters
            .iter()
            .map(|parameter| Value::String(parameter.name.clone()))
            .collect();
        metadata.insert(String::from("parameterNames"), Value::Array(names));
        Ok(metadata)
    }

    fn add_method_marker(
        written: &WrittenPolicies<'_>,
        metadata: &mut Map<String, Value>,
        member: &CppMember,
        marker: &CppAttribute,
    ) -> Result<(), Failure> {
        let (key, value) = written.method(&Self::written(marker), member.parameters.len())?;
        let fresh = metadata.insert(key.to_owned(), value).is_none();
        fresh
            .then_some(())
            .ok_or_else(|| written.failure(marker.at, Self::TWICE))
    }

    fn add_parameter_marker(
        written: &WrittenPolicies<'_>,
        metadata: &mut Map<String, Value>,
        index: usize,
        marker: &CppAttribute,
    ) -> Result<Option<Value>, Failure> {
        if marker.name == "CacheKey" {
            return Self::cache_key(written, index, marker).map(Some);
        }
        Self::page_marker(written, metadata, index, marker).map(|()| None)
    }

    fn cache_key(
        written: &WrittenPolicies<'_>,
        index: usize,
        marker: &CppAttribute,
    ) -> Result<Value, Failure> {
        (!marker.has_arguments())
            .then(|| json!(index))
            .ok_or_else(|| written.failure(marker.at, Self::KEY_FORM))
    }

    fn page_marker(
        written: &WrittenPolicies<'_>,
        metadata: &mut Map<String, Value>,
        index: usize,
        marker: &CppAttribute,
    ) -> Result<(), Failure> {
        marker
            .has_arguments()
            .then_some(())
            .ok_or_else(|| written.failure(marker.at, Self::BARE_PAGE))?;
        let page = written.page(&Self::written(marker), index)?;
        let fresh = metadata.insert(String::from("pageable"), page).is_none();
        fresh
            .then_some(())
            .ok_or_else(|| written.failure(marker.at, Self::TWICE))
    }

    fn add_keys(
        written: &WrittenPolicies<'_>,
        metadata: &mut Map<String, Value>,
        member: &CppMember,
        keys: Vec<Value>,
    ) -> Result<(), Failure> {
        if keys.is_empty() {
            return Ok(());
        }
        let cacheable = metadata
            .get_mut("cacheable")
            .ok_or_else(|| written.failure(member.name_at, Self::NEEDS_CACHED))?;
        cacheable["keyArgs"] = Value::Array(keys);
        Ok(())
    }

    fn yon_shape(source: &str, member: &CppMember, markers: CppMarked<'_>) -> bool {
        [
            Self::service_shape(source, member, markers),
            !member.is_task(),
        ] == [true, true]
    }

    fn service_shape(source: &str, member: &CppMember, (methods, _): CppMarked<'_>) -> bool {
        let has_body = source.as_bytes().get(member.terminator) == Some(&b'{');
        let named = member
            .parameters
            .iter()
            .all(|parameter| !parameter.name.is_empty());
        let transacted = methods
            .iter()
            .any(|marker| marker.name == Self::TRANSACTION);
        [
            member.is_instance(CppMemberKind::Method),
            has_body,
            [member.is_task(), transacted] != [true, true],
            !member.type_text.contains('&'),
            named,
        ] == [true; 5]
    }

    fn client_shape(_: &str, member: &CppMember, (methods, parameters): CppMarked<'_>) -> bool {
        let requests = methods
            .iter()
            .all(|marker| Self::CLIENT_MARKERS.contains(&marker.name.as_str()));
        let keys = parameters
            .iter()
            .all(|(_, marker)| marker.name == "CacheKey");
        [member.is_task(), requests, keys] == [true; 3]
    }

    fn policy(
        written: &WrittenPolicies<'_>,
        class: &CppClass,
        name: &str,
        metadata: &Map<String, Value>,
        cache: Option<&Value>,
        module: &str,
    ) -> Result<Value, Failure> {
        let at = class
            .members
            .iter()
            .find(|member| member.name == name)
            .map_or(class.name_at, |member| member.name_at);
        let caching = ["cacheable", "cachePut", "cacheEvict"]
            .iter()
            .any(|key| metadata.contains_key(*key));
        ([caching, cache.is_none()] != [true, true])
            .then_some(())
            .ok_or_else(|| written.failure(at, Self::NEEDS_CACHE))?;
        let returns_void = class.members.iter().any(|member| {
            [
                member.name == name,
                super::CppRemote::answer_type(member) == "void",
            ] == [true, true]
        });
        let keeps_value = ["cacheable", "cachePut"]
            .iter()
            .any(|key| metadata.contains_key(*key));
        ([keeps_value, returns_void] != [true, true])
            .then_some(())
            .ok_or_else(|| written.failure(at, Self::RETURNS))?;
        let mutation = ["cachePut", "cacheEvict"]
            .iter()
            .find_map(|key| metadata.get(*key).map(|policy| (*key, policy)))
            .map(|(key, policy)| {
                let operation = ["cache:evict", "cache:put"][usize::from(key == "cachePut")];
                json!({
                    "operation": operation,
                    "method": policy["target"]["method"],
                    "arguments": policy["arguments"],
                    "keyArgs": policy["target"]["cacheable"]["keyArgs"],
                    "pageable": policy["target"].get("pageable").cloned().unwrap_or(Value::Null),
                })
            });
        let limit = metadata
            .get("concurrencyLimit")
            .map(|limit| json!({"limit": limit["limit"], "blocking": limit["policy"] == "block"}));
        Ok(json!({
            "partition": Self::PARTITION,
            "module": module,
            "owner": class.name,
            "name": name,
            "cache": cache.cloned().unwrap_or(Value::Null),
            "keyArgs": metadata.get("cacheable").map_or(Value::Null, |cached| cached["keyArgs"].clone()),
            "pageable": metadata.get("pageable").cloned().unwrap_or(Value::Null),
            "retry": metadata.get("retryable").cloned().unwrap_or(Value::Null),
            "limit": limit.unwrap_or(Value::Null),
            "mutation": mutation.unwrap_or(Value::Null),
            "transactional": metadata.get("transactional").cloned().unwrap_or(Value::Null),
        }))
    }
}
