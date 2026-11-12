use std::path::{Path, PathBuf};

pub(super) struct KotlinUnitFiles;

impl KotlinUnitFiles {
    const FILES: [(&'static str, &'static str); 2] = [
        (
            "BehaviorTasks.kt",
            concat!(
                "import kotlin.coroutines.startCoroutine\n\n",
                include_str!("kotlin_test_tasks.kt")
            ),
        ),
        (
            "TacReads.kt",
            concat!(
                include_str!("../native/prelude_reads.kt"),
                include_str!("../native/prelude_jvm_models.kt"),
                "\nfun tacModelOrder(model: String): List<String>? = null\n"
            ),
        ),
    ];
    const BROWSER_FAKES: &'static str = include_str!("kotlin_browser_fakes.kt");
    const WEB_APIS: [(&'static str, &'static str); 13] = [
        ("Clock", include_str!("../native/web_apis/Clock.kt")),
        (
            "SecureRandom",
            include_str!("../native/web_apis/SecureRandom.kt"),
        ),
        (
            "Connectivity",
            include_str!("../native/web_apis/Connectivity.kt"),
        ),
        (
            "PageVisibility",
            include_str!("../native/web_apis/PageVisibility.kt"),
        ),
        (
            "Preferences",
            include_str!("../native/web_apis/Preferences.kt"),
        ),
        ("Clipboard", include_str!("../native/web_apis/Clipboard.kt")),
        ("Share", include_str!("../native/web_apis/Share.kt")),
        ("Files", include_str!("../native/web_apis/Files.kt")),
        (
            "Notifications",
            include_str!("../native/web_apis/Notifications.kt"),
        ),
        ("Push", include_str!("../native/web_apis/Push.kt")),
        (
            "Geolocation",
            include_str!("../native/web_apis/Geolocation.kt"),
        ),
        (
            "WindowControls",
            include_str!("../native/web_apis/WindowControls.kt"),
        ),
        ("Focus", include_str!("../native/web_apis/Focus.kt")),
    ];
    const LATER_APIS: [(&'static str, &'static str); 2] = [
        (
            "StorageSpace",
            include_str!("../native/web_apis/StorageSpace.kt"),
        ),
        ("Passkeys", include_str!("../native/web_apis/Passkeys.kt")),
    ];

    pub(super) fn write(directory: &Path) -> std::io::Result<Vec<PathBuf>> {
        Self::FILES
            .iter()
            .map(|(name, text)| {
                let path = directory.join(name);
                std::fs::write(&path, text).map(|()| path)
            })
            .collect()
    }

    pub(super) fn browser(unit: &str) -> String {
        let apis: String = Self::WEB_APIS
            .iter()
            .chain(Self::LATER_APIS.iter())
            .filter(|(name, _)| unit.contains(name))
            .map(|(_, source)| *source)
            .collect();
        Self::with_fakes(&apis, unit)
    }

    fn with_fakes(apis: &str, unit: &str) -> String {
        let unused = [apis.is_empty(), unit.contains("BrowserFakes")] == [true, false];
        if unused {
            return String::new();
        }
        format!("{}{apis}", Self::BROWSER_FAKES)
    }
}
