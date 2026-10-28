pub(crate) struct RenamedAnnotation;

impl RenamedAnnotation {
    pub(crate) const NAMES: [(&str, &str); 40] = [
        ("Autowired", "Needs"),
        ("Inject", "Needs"),
        ("RequiredArgsConstructor", "NeedsAll"),
        ("Qualifier", "Named"),
        ("Scope", "Lifetime"),
        ("Repository", "Store"),
        ("Mapper", "Converter"),
        ("Mapping", "Convert"),
        ("Configuration", "Setup"),
        ("ConfigurationProperties", "Settings"),
        ("Value", "Setting"),
        ("Profile", "WhenProfile"),
        ("ConditionalOnProperty", "WhenSetting"),
        ("PostConstruct", "OnStart"),
        ("PreDestroy", "OnStop"),
        ("EventListener", "OnEvent"),
        ("Async", "InBackground"),
        ("Scheduled", "Schedule"),
        ("Cacheable", "Cached"),
        ("CachePut", "UpdateCache"),
        ("CacheEvict", "ClearCache"),
        ("CacheConfig", "CacheSettings"),
        ("Retryable", "Retry"),
        ("ConcurrencyLimit", "MaxAtOnce"),
        ("Transactional", "AllOrNothing"),
        ("PageableDefault", "PageDefaults"),
        ("Validated", "Checked"),
        ("Valid", "Check"),
        ("NotNull", "Required"),
        ("NotBlank", "NotEmpty"),
        ("Size", "Length"),
        ("TachyonTest", "Test"),
        ("TestConfiguration", "TestSetup"),
        ("MockBean", "Fake"),
        ("MockitoBean", "Fake"),
        ("ActiveProfiles", "UseProfiles"),
        ("TestPropertySource", "TestSettings"),
        ("DirtiesContext", "FreshStart"),
        ("Component", "Delegate"),
        ("Helper", "Delegate"),
    ];

    pub(crate) fn new_name(authored: &str) -> Option<&'static str> {
        let normalized = Self::normalized(Self::last_segment(authored));
        for (old, new) in Self::NAMES {
            if old.to_ascii_lowercase() == normalized {
                return Some(new);
            }
        }
        None
    }

    pub(crate) fn spelled_like(authored: &str, new: &str) -> String {
        let authored = Self::last_segment(authored);
        if authored.starts_with(|character: char| character.is_ascii_uppercase()) {
            return new.to_owned();
        }
        if authored.contains('_')
            || !authored.contains(|character: char| character.is_ascii_uppercase())
        {
            return Self::snake_case(new);
        }
        let mut characters = new.chars();
        let first = characters.next().unwrap_or_default().to_ascii_lowercase();
        format!("{first}{}", characters.as_str())
    }

    pub(crate) fn message(authored: &str) -> Option<String> {
        let new = Self::new_name(authored)?;
        let old = Self::last_segment(authored);
        let written = Self::spelled_like(authored, new);
        Some(format!(
            "'{old}' was renamed to {written}; write {written}. Run `ty migrate fix --write` to rename every old annotation"
        ))
    }

    pub(crate) fn snake_case(pascal: &str) -> String {
        let mut result = String::new();
        for (index, character) in pascal.chars().enumerate() {
            if character.is_ascii_uppercase() && index > 0 {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
        }
        result
    }

    fn last_segment(name: &str) -> &str {
        name.rsplit("::").next().unwrap_or(name)
    }

    fn normalized(name: &str) -> String {
        name.replace('_', "").to_ascii_lowercase()
    }
}

pub(crate) struct RoutingRename;

impl RoutingRename {
    const CONTROLLER: &'static str = "it marks a controller and gives the path its routes start with, like @RequestRoute(\"/orders\")";
    const CLIENT: &'static str = "it marks a class that sends HTTP requests and gives the path they start with, like @HTTPClient(\"/api/orders\")";
    const CONTEXT_NAMES: [&'static str; 5] = [
        "Controller",
        "Client",
        "WebCall",
        "CookieValue",
        "RequestPart",
    ];
    const NAMES: [(&'static str, &'static str, &'static str); 18] = [
        ("Controller", "RequestRoute", Self::CONTROLLER),
        ("RequestMapping", "RequestRoute", Self::CONTROLLER),
        (
            "GetMapping",
            "GetRoute",
            "it means this method answers GET requests",
        ),
        (
            "PostMapping",
            "PostRoute",
            "it means this method answers POST requests",
        ),
        (
            "PutMapping",
            "PutRoute",
            "it means this method answers PUT requests",
        ),
        (
            "PatchMapping",
            "PatchRoute",
            "it means this method answers PATCH requests",
        ),
        (
            "DeleteMapping",
            "DeleteRoute",
            "it means this method answers DELETE requests",
        ),
        ("Client", "HTTPClient", Self::CLIENT),
        ("HttpExchange", "HTTPClient", Self::CLIENT),
        (
            "GetExchange",
            "GetRequest",
            "it means calling this method sends a GET request",
        ),
        (
            "PostExchange",
            "PostRequest",
            "it means calling this method sends a POST request",
        ),
        (
            "PutExchange",
            "PutRequest",
            "it means calling this method sends a PUT request",
        ),
        (
            "PatchExchange",
            "PatchRequest",
            "it means calling this method sends a PATCH request",
        ),
        (
            "DeleteExchange",
            "DeleteRequest",
            "it means calling this method sends a DELETE request",
        ),
        (
            "WebCall",
            "GetRequest",
            "write @GetRequest, @PostRequest, @PutRequest, @PatchRequest or @DeleteRequest with the path inside, like @GetRequest(\"/products\"): @GetRequest means calling the method sends a GET request",
        ),
        (
            "CookieValue",
            "FromCookie",
            "on a controller it means this argument is the value of a cookie",
        ),
        ("RestController", "RequestRoute", Self::CONTROLLER),
        (
            "RequestPart",
            "FromBody",
            "on a controller it means this argument is the request body",
        ),
    ];

    pub(crate) fn message(authored: &str) -> Option<String> {
        let last = authored.rsplit("::").next().unwrap_or(authored);
        let normalized = last.replace('_', "").to_ascii_lowercase();
        for (old, new, meaning) in Self::NAMES {
            if old.to_ascii_lowercase() != normalized {
                continue;
            }
            let written = Self::spelled_like(last, new);
            return Some(format!(
                "'@{last}' is written @{written} in Tachyon: {meaning}. Run `ty migrate fix --write` to convert every old route and client"
            ));
        }
        None
    }

    pub(crate) fn direct(authored: &str) -> Option<&'static str> {
        let normalized = authored.replace('_', "").to_ascii_lowercase();
        Self::NAMES
            .iter()
            .filter(|(old, ..)| !Self::CONTEXT_NAMES.contains(old))
            .find(|(old, ..)| old.to_ascii_lowercase() == normalized)
            .map(|(_, new, _)| *new)
    }

    pub(crate) fn snake_case(name: &str) -> String {
        let characters: Vec<char> = name.chars().collect();
        let mut result = String::new();
        for (index, character) in characters.iter().enumerate() {
            let previous = index.checked_sub(1).and_then(|at| characters.get(at));
            let next = characters.get(index + 1);
            let starts_word = character.is_ascii_uppercase()
                && index > 0
                && (previous.is_some_and(char::is_ascii_lowercase)
                    || next.is_some_and(char::is_ascii_lowercase));
            if starts_word {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
        }
        result
    }

    pub(crate) fn spelled_like(authored: &str, new: &str) -> String {
        if authored.starts_with(|character: char| character.is_ascii_uppercase()) {
            return new.to_owned();
        }
        Self::snake_case(new)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RenamedAnnotation;

    #[test]
    fn every_spelling_of_an_old_name_finds_the_new_name() {
        for authored in ["Autowired", "autowired", "yon::autowired", "tac::Autowired"] {
            assert_eq!(RenamedAnnotation::new_name(authored), Some("Needs"));
        }
        assert_eq!(
            RenamedAnnotation::new_name("required_args_constructor"),
            Some("NeedsAll")
        );
        assert_eq!(RenamedAnnotation::new_name("Needs"), None);
    }

    #[test]
    fn the_new_name_follows_the_authored_spelling() {
        assert_eq!(
            RenamedAnnotation::spelled_like("PostConstruct", "OnStart"),
            "OnStart"
        );
        assert_eq!(
            RenamedAnnotation::spelled_like("post_construct", "OnStart"),
            "on_start"
        );
        assert_eq!(
            RenamedAnnotation::spelled_like("yon::cacheconfig", "CacheSettings"),
            "cache_settings"
        );
        assert_eq!(
            RenamedAnnotation::spelled_like("postConstruct", "OnStart"),
            "onStart"
        );
    }

    #[test]
    fn the_message_names_the_new_annotation_and_the_migration() {
        assert_eq!(RenamedAnnotation::message("Lifetime"), None);
        let message = RenamedAnnotation::message("Scope").unwrap_or_default();
        assert!(
            message.starts_with("'Scope' was renamed to Lifetime; write Lifetime."),
            "{message}"
        );
        assert!(message.contains("ty migrate fix --write"), "{message}");
    }

    fn lowering_error(file: &str, source: &str) -> String {
        let failure =
            crate::behavior::lower(std::path::Path::new(file), source, "yon").expect_err(source);
        failure.diagnostics()[0].message.clone()
    }

    #[test]
    fn lowering_refuses_an_old_name_in_javascript_and_in_every_other_language() {
        let javascript = lowering_error("yon.js", "class Orders {\n@Autowired\nstore;\n}\n");
        assert!(javascript.contains("renamed to Needs"), "{javascript}");
        let python = lowering_error("yon.py", "@cache_config({})\nclass Orders:\n    pass\n");
        assert!(python.contains("renamed to cache_settings"), "{python}");
        let csharp = lowering_error("yon.cs", "[Transactional({})]\npublic class Orders {}\n");
        assert!(csharp.contains("renamed to AllOrNothing"), "{csharp}");
    }

    #[test]
    fn needs_on_a_yon_class_is_left_to_the_yon_provider_graph() {
        let source =
            "@needs(MoneyService)\nclass MoneyController:\n    def GET(self):\n        return {}\n";
        crate::behavior::lower(std::path::Path::new("yon.py"), source, "yon")
            .expect("class-level Needs lists providers");
    }

    #[test]
    fn a_junit_style_test_marker_on_a_method_is_refused() {
        let javascript = crate::behavior::lower(
            std::path::Path::new("yon.js"),
            "class Orders {\n@Test\ntestTotal() {}\n}\n",
            "yon",
        );
        assert!(javascript.is_err(), "{javascript:?}");
        let csharp = crate::behavior::lower(
            std::path::Path::new("yon.cs"),
            "public class Orders {\n    [SetUp]\n    public void Prepare() {}\n}\n",
            "yon",
        );
        assert!(csharp.is_err(), "{csharp:?}");
    }

    #[test]
    fn an_old_route_or_client_name_explains_the_new_one() {
        use super::RoutingRename;
        let controller = RoutingRename::message("Controller").unwrap_or_default();
        assert!(
            controller.starts_with(
                "'@Controller' is written @RequestRoute in Tachyon: it marks a controller"
            ),
            "{controller}"
        );
        let python = RoutingRename::message("post_mapping").unwrap_or_default();
        assert!(python.contains("@post_route"), "{python}");
        assert!(python.contains("answers POST requests"), "{python}");
        let client = RoutingRename::message("HTTPExchange").unwrap_or_default();
        assert!(client.contains("@HTTPClient"), "{client}");
        assert_eq!(RoutingRename::snake_case("HTTPClient"), "http_client");
        assert_eq!(RoutingRename::snake_case("GetRequest"), "get_request");
        assert_eq!(RoutingRename::message("RequestRoute"), None);
    }
}
