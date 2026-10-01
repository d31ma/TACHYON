#[derive(Debug, Eq, PartialEq)]
pub(super) enum JavaConstructorProvider {
    Preference(String),
    Publisher(String),
}

impl JavaConstructorProvider {
    pub(super) fn read(kind: &str, name: &str) -> Option<Self> {
        Self::typed(kind, name)
    }

    pub(super) fn name(&self) -> &str {
        self.named()
    }

    pub(super) fn expression(&self) -> &'static str {
        self.constructed()
    }

    fn typed(kind: &str, name: &str) -> Option<Self> {
        match kind {
            "PreferenceStrings" => Some(Self::Preference(String::from(name))),
            "ApplicationEventPublisher" => Some(Self::Publisher(String::from(name))),
            _ => None,
        }
    }

    fn named(&self) -> &str {
        match self {
            Self::Preference(name) | Self::Publisher(name) => name,
        }
    }

    fn constructed(&self) -> &'static str {
        match self {
            Self::Preference(_) => "new BrowserPreferenceStrings(preferences)",
            Self::Publisher(_) => "new ApplicationEventPublisher(owner)",
        }
    }
}
