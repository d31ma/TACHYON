pub(crate) struct KotlinBuiltins;

impl KotlinBuiltins {
    pub(super) const NONE: &'static str = "\n@Suppress(\"UNUSED_PARAMETER\")\nprivate fun behaviorBuiltinCache(name: String): Behaviors.CacheAdapter? = null\n\n@Suppress(\"UNUSED_PARAMETER\")\nprivate fun behaviorBuiltinEntities(name: String): Behaviors.EntityAdapter? = null\n";

    pub(crate) fn in_browser(lowered: &str, browser: &str) -> String {
        lowered.replace(Self::NONE, &format!("\n{browser}"))
    }
}
