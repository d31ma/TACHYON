pub(super) struct SwiftClientAnswer<'a> {
    returned: &'a str,
    declared_exchange: bool,
}

impl<'a> SwiftClientAnswer<'a> {
    const BEHAVIOR_JSON: &'static str = "BehaviorJSON";
    const VOID: &'static str = "Void";
    const UNIT: &'static str = "()";

    pub(super) fn of(returned: &'a str, declared_exchange: bool) -> Self {
        Self {
            returned,
            declared_exchange,
        }
    }

    pub(super) fn allowed(&self) -> bool {
        [self.returned == Self::BEHAVIOR_JSON, self.declared_exchange].contains(&true)
    }

    pub(super) fn written(&self, call: &str) -> String {
        self.untyped(call).unwrap_or_else(|| self.typed(call))
    }

    fn untyped(&self, call: &str) -> Option<String> {
        match self.returned {
            Self::BEHAVIOR_JSON => Some(format!("return {call}")),
            "" | Self::VOID | Self::UNIT => Some(format!("_ = {call}")),
            _ => None,
        }
    }

    fn typed(&self, call: &str) -> String {
        format!(
            "let answer = {call}\nreturn try Behaviors.answer({}.self, answer)",
            self.returned
        )
    }
}
