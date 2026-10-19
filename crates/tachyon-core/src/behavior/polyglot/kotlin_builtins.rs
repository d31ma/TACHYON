pub(crate) struct KotlinBuiltins;

impl KotlinBuiltins {
    pub(super) const NONE: &'static str = "\n@Suppress(\"UNUSED_PARAMETER\")\nprivate fun behaviorBuiltinCache(name: String): Behaviors.CacheAdapter? = null\n\n@Suppress(\"UNUSED_PARAMETER\")\nprivate fun behaviorBuiltinEntities(name: String): Behaviors.EntityAdapter? = null\n\n@Suppress(\"UNUSED_PARAMETER\", \"RedundantSuspendModifier\")\nprivate suspend fun behaviorDefaultExchange(request: Map<String, Any?>): Any? = throw Behaviors.Failure(\"HTTP_MISSING\", null, \"The default HTTP transport runs only in the browser. Give this client an HTTP adapter in its Setup.\")\n";

    pub(crate) fn in_browser(lowered: &str, browser: &str) -> String {
        lowered.replace(Self::NONE, &format!("\n{browser}"))
    }

    pub(crate) fn natively(lowered: &str, linked: &str) -> String {
        lowered.replace(Self::NONE, &format!("\n{linked}"))
    }
}
