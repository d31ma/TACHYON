pub(super) struct SwiftClientAnswer<'a> {
    returned: &'a str,
    declared_exchange: bool,
}

impl<'a> SwiftClientAnswer<'a> {
    const BEHAVIOR_JSON: &'static str = "BehaviorJSON";
    const NO_ANSWER: [&'static str; 3] = ["", "Void", "()"];

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
        if self.returned == Self::BEHAVIOR_JSON {
            return format!("return {call}");
        }
        if Self::NO_ANSWER.contains(&self.returned) {
            return format!("_ = {call}");
        }
        format!(
            "let answer = {call}\nreturn try Behaviors.answer({}.self, answer)",
            self.returned
        )
    }
}
