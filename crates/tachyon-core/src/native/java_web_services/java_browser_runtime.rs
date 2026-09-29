pub(super) struct JavaBrowserRuntime;

impl JavaBrowserRuntime {
    const BUILTINS: &'static str = include_str!("BehaviorBuiltins.java");
    const TEA_VM_SWAPS: [(&'static str, &'static str); 6] = [
        ("java.util.concurrent.Semaphore", "TacPermits"),
        ("java.util.Map.copyOf(", "TacMaps.copyOf("),
        ("java.util.StringJoiner", "TacJoiner"),
        ("Thread.sleep(", "TacPause.sleep("),
        ("synchronized (this) ", ""),
        ("static synchronized ", "static "),
    ];

    pub(super) fn adapt(lowered: &str) -> String {
        let browser = crate::behavior::JavaBuiltins::in_browser(lowered, Self::BUILTINS);
        Self::TEA_VM_SWAPS
            .iter()
            .fold(browser, |text, (missing, present)| {
                text.replace(missing, present)
            })
    }
}
