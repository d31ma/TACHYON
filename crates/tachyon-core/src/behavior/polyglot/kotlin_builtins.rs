pub(crate) struct KotlinBuiltins;

impl KotlinBuiltins {
    pub(super) const NONE: &'static str = "\n@Suppress(\"UNUSED_PARAMETER\")\nprivate fun behaviorBuiltinCache(name: String): Behaviors.CacheAdapter? = null\n\n@Suppress(\"UNUSED_PARAMETER\")\nprivate fun behaviorBuiltinEntities(name: String): Behaviors.EntityAdapter? = null\n\n@Suppress(\"UNUSED_PARAMETER\", \"RedundantSuspendModifier\")\nprivate suspend fun behaviorDefaultExchange(request: Map<String, Any?>): Any? = throw Behaviors.Failure(\"HTTP_MISSING\", null, \"The default HTTP transport runs only in the browser. Give this client an HTTP adapter in its Setup.\")\n";

    const MODEL_READ: &'static str = "tacRead<";
    const ERROR_BODY: &'static str = "\ninline fun <reified T> Behaviors.errorBody(error: Throwable?): T? = errorBody(error) { tacRead<T>(it) }\n";

    pub(super) fn with_error_body(lowered: String) -> String {
        let error_body = Self::error_body_for(&lowered);
        lowered + error_body
    }

    fn error_body_for(lowered: &str) -> &'static str {
        if lowered.contains(Self::MODEL_READ) {
            return Self::ERROR_BODY;
        }
        ""
    }

    pub(crate) fn in_browser(lowered: &str, browser: &str) -> String {
        lowered.replace(Self::NONE, &format!("\n{browser}"))
    }

    pub(crate) fn natively(lowered: &str, linked: &str) -> String {
        lowered.replace(Self::NONE, &format!("\n{linked}"))
    }
}
