#[derive(Debug, Eq, PartialEq)]
pub(super) enum JavaConstructorProvider {
    Preference(String),
    Publisher(String),
    Service {
        name: String,
        instance: String,
        class: String,
    },
}

impl JavaConstructorProvider {
    pub(super) fn read(kind: &str, name: &str) -> Option<Self> {
        Self::typed(kind, name)
    }

    pub(super) fn name(&self) -> &str {
        self.named()
    }

    pub(super) fn expression(&self) -> String {
        self.constructed()
    }

    pub(super) fn needed(&self) -> Option<(&str, &str)> {
        self.target()
    }

    fn target(&self) -> Option<(&str, &str)> {
        match self {
            Self::Service {
                instance, class, ..
            } => Some((instance, class)),
            _ => None,
        }
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
            Self::Preference(name) | Self::Publisher(name) | Self::Service { name, .. } => name,
        }
    }

    fn constructed(&self) -> String {
        match self {
            Self::Preference(_) => String::from("new BrowserPreferenceStrings(preferences)"),
            Self::Publisher(_) => String::from("new ApplicationEventPublisher(owner)"),
            Self::Service {
                instance, class, ..
            } => format!(
                "({class}) TacNeeds.need({})",
                serde_json::to_string(instance).unwrap_or_default()
            ),
        }
    }
}
