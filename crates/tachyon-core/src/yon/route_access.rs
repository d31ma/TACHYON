#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum RouteAccess {
    #[default]
    Missing,
    Public,
    SignedIn,
    Allow(Vec<String>),
}

impl RouteAccess {
    #[must_use]
    pub(crate) fn or_class(self, class: &Self) -> Self {
        self.chosen_over(class)
    }

    #[must_use]
    pub(crate) fn is_missing(&self) -> bool {
        *self == Self::Missing
    }

    #[must_use]
    pub(crate) fn needs_identity(&self) -> bool {
        self.asks_for_identity()
    }

    #[must_use]
    pub(crate) fn roles(&self) -> &[String] {
        self.allowed_roles()
    }

    fn chosen_over(self, class: &Self) -> Self {
        if self.is_missing() {
            return class.clone();
        }
        self
    }

    fn asks_for_identity(&self) -> bool {
        matches!(self, Self::SignedIn | Self::Allow(_))
    }

    fn allowed_roles(&self) -> &[String] {
        match self {
            Self::Allow(roles) => roles,
            _ => &[],
        }
    }
}
