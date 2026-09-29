pub(crate) struct JavaBuiltins;

impl JavaBuiltins {
    pub(super) const NONE: &'static str = "\nfinal class BehaviorBuiltins {\n    private BehaviorBuiltins() {}\n    static Behaviors.CacheAdapter cache(String name) { return null; }\n    static Behaviors.EntityAdapter entities(String name) { return null; }\n}\n";

    pub(crate) fn in_browser(lowered: &str, browser: &str) -> String {
        lowered.replace(Self::NONE, &format!("\n{browser}"))
    }
}
