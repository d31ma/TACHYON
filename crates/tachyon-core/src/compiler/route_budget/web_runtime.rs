use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WebRuntime {
    pub(super) name: &'static str,
    shared_module: &'static str,
    pub(super) kilobytes: u64,
}

impl WebRuntime {
    const SERVICE_FOLDER: &'static str = ".tachyon/services/";
    const ALL: [Self; 10] = [
        Self::new(".NET", "tachyon-csharp.js", 96),
        Self::new("Rust", "tachyon-rust.js", 92),
        Self::new("Ruby", "tachyon-ruby-services.js", 20),
        Self::new("Lua", "tachyon-lua-services.js", 12),
        Self::new("Java", "tachyon-java.js", 12),
        Self::new("Go", "tachyon-go.js", 8),
        Self::new("Dart", "tachyon-dart.js", 8),
        Self::new("Kotlin", "tachyon-kotlin.js", 8),
        Self::new("C++", "tachyon-cpp.js", 4),
        Self::new("Swift", "tachyon-swift.js", 4),
    ];

    const fn new(name: &'static str, shared_module: &'static str, kilobytes: u64) -> Self {
        Self {
            name,
            shared_module,
            kilobytes,
        }
    }

    pub(super) fn reached(loaded: &BTreeSet<String>) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|runtime| runtime.is_loaded(loaded))
            .collect()
    }

    fn is_loaded(self, loaded: &BTreeSet<String>) -> bool {
        let path = format!("{}{}", Self::SERVICE_FOLDER, self.shared_module);
        loaded.contains(&path)
    }
}
