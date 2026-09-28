use super::Choice;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppTarget {
    Web,
    Macos,
    Windows,
    Linux,
    Ios,
    Android,
}

impl AppTarget {
    const NAMES: [&'static str; 6] = [
        "web,browser",
        "macos,mac",
        "windows,win",
        "linux",
        "ios",
        "android",
    ];
    const LABELS: [&'static str; 6] = [
        "Web: runs in a browser",
        "macOS: a desktop app for Mac",
        "Windows: a desktop app for Windows",
        "Linux: a desktop app for Linux",
        "iOS: an app for iPhone and iPad",
        "Android: an app for Android phones",
    ];

    #[must_use]
    pub fn is_native(self) -> bool {
        self != Self::Web
    }

    pub(crate) fn bundle_script(self) -> String {
        format!(
            "    \"bundle:{0}\": \"ty bundle --target {0}\",\n",
            self.flag_name()
        )
    }
}

impl Choice for AppTarget {
    const ALL: &'static [Self] = &[
        Self::Web,
        Self::Macos,
        Self::Windows,
        Self::Linux,
        Self::Ios,
        Self::Android,
    ];
    const QUESTION: &'static str = "Where should the app run? You can choose more than one.";

    fn names(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn label(self) -> &'static str {
        Self::LABELS[self as usize]
    }
}
