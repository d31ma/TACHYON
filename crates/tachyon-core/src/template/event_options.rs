use serde_json::{Map, Value};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EventOptions {
    pub(crate) listen_on: Option<&'static str>,
    pub(crate) key: Option<&'static str>,
    pub(crate) modifier_keys: Option<Vec<&'static str>>,
    pub(crate) prevent: bool,
    pub(crate) stop: bool,
    pub(crate) outside: bool,
}

impl EventOptions {
    const KEYS: [(&'static str, &'static str); 10] = [
        ("escape", "Escape"),
        ("enter", "Enter"),
        ("space", " "),
        ("tab", "Tab"),
        ("up", "ArrowUp"),
        ("down", "ArrowDown"),
        ("left", "ArrowLeft"),
        ("right", "ArrowRight"),
        ("home", "Home"),
        ("end", "End"),
    ];
    const MODIFIER_KEYS: [(&'static str, &'static str); 4] = [
        ("shift", "shiftKey"),
        ("ctrl", "ctrlKey"),
        ("alt", "altKey"),
        ("meta", "metaKey"),
    ];
    const PREVENT: &'static str = "prevent";
    const STOP: &'static str = "stop";
    const OUTSIDE: &'static str = "outside";
    const UNKNOWN_MESSAGE: &'static str = "Event modifier must be one of escape, enter, space, tab, up, down, left, right, home, end, shift, ctrl, alt, meta, prevent, stop or outside.";
    const REPEATED_MESSAGE: &'static str = "Event modifier may appear only once.";
    const ONE_KEY_MESSAGE: &'static str = "Event binding may filter one key only.";
    const OUTSIDE_TARGET_MESSAGE: &'static str =
        "The outside modifier cannot be used with a document: or window: listener.";

    pub(crate) fn parse(
        listen_on: Option<&'static str>,
        modifiers: &[&str],
    ) -> Result<Self, &'static str> {
        Self::refuse_unknown(modifiers)?;
        Self::refuse_repeated(modifiers)?;
        let key = Self::key(modifiers)?;
        let options = Self {
            listen_on,
            key,
            modifier_keys: Self::exact_modifier_keys(key, Self::pressed_keys(modifiers)),
            prevent: modifiers.contains(&Self::PREVENT),
            stop: modifiers.contains(&Self::STOP),
            outside: modifiers.contains(&Self::OUTSIDE),
        };
        options.refuse_outside_target()?;
        Ok(options)
    }

    pub(crate) fn members(&self) -> Map<String, Value> {
        let mut members = Map::new();
        Self::insert_text(&mut members, "listen", self.listen_on);
        Self::insert_text(&mut members, "key", self.key);
        Self::insert_list(&mut members, "modifierKeys", self.modifier_keys.as_ref());
        Self::insert_flag(&mut members, Self::PREVENT, self.prevent);
        Self::insert_flag(&mut members, Self::STOP, self.stop);
        Self::insert_flag(&mut members, Self::OUTSIDE, self.outside);
        members
    }

    fn refuse_unknown(modifiers: &[&str]) -> Result<(), &'static str> {
        let known = modifiers.iter().all(|modifier| Self::is_known(modifier));
        known.then_some(()).ok_or(Self::UNKNOWN_MESSAGE)
    }

    fn is_known(modifier: &str) -> bool {
        Self::KEYS.iter().any(|(name, _)| *name == modifier)
            || Self::MODIFIER_KEYS
                .iter()
                .any(|(name, _)| *name == modifier)
            || [Self::PREVENT, Self::STOP, Self::OUTSIDE].contains(&modifier)
    }

    fn refuse_repeated(modifiers: &[&str]) -> Result<(), &'static str> {
        let unique = modifiers.iter().collect::<BTreeSet<_>>();
        (unique.len() == modifiers.len())
            .then_some(())
            .ok_or(Self::REPEATED_MESSAGE)
    }

    fn key(modifiers: &[&str]) -> Result<Option<&'static str>, &'static str> {
        let keys = Self::selected(&Self::KEYS, modifiers);
        (keys.len() <= 1)
            .then(|| keys.first().copied())
            .ok_or(Self::ONE_KEY_MESSAGE)
    }

    fn pressed_keys(modifiers: &[&str]) -> Vec<&'static str> {
        Self::selected(&Self::MODIFIER_KEYS, modifiers)
    }

    fn selected(table: &[(&'static str, &'static str)], modifiers: &[&str]) -> Vec<&'static str> {
        table
            .iter()
            .filter(|(name, _)| modifiers.contains(name))
            .map(|(_, value)| *value)
            .collect()
    }

    fn exact_modifier_keys(
        key: Option<&'static str>,
        pressed: Vec<&'static str>,
    ) -> Option<Vec<&'static str>> {
        (key.is_some() || !pressed.is_empty()).then_some(pressed)
    }

    fn refuse_outside_target(&self) -> Result<(), &'static str> {
        let refused = self.outside && self.listen_on.is_some();
        (!refused).then_some(()).ok_or(Self::OUTSIDE_TARGET_MESSAGE)
    }

    fn insert_text(members: &mut Map<String, Value>, name: &str, value: Option<&'static str>) {
        if let Some(value) = value {
            members.insert(String::from(name), Value::from(value));
        }
    }

    fn insert_list(
        members: &mut Map<String, Value>,
        name: &str,
        value: Option<&Vec<&'static str>>,
    ) {
        if let Some(value) = value {
            members.insert(String::from(name), Value::from(value.clone()));
        }
    }

    fn insert_flag(members: &mut Map<String, Value>, name: &str, value: bool) {
        if value {
            members.insert(String::from(name), Value::Bool(true));
        }
    }
}
