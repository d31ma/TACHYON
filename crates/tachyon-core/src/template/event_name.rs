use super::event_options::EventOptions;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EventName {
    pub(crate) event_type: String,
    pub(crate) options: EventOptions,
}

impl EventName {
    pub(crate) const HELP: &'static str = "Write on:[document:|window:]<event>[.modifier], as in on:keydown.escape=\"close()\" or on:click.outside=\"close()\".";
    const MAX_BYTES: usize = 64;
    const TARGETS: [&'static str; 2] = ["document", "window"];
    const KEY_EVENTS: [&'static str; 2] = ["keydown", "keyup"];
    const NAME_MESSAGE: &'static str = "Event name must be 1 to 64 characters of a-z, 0-9, ':' and '-', and must not start or end with '-'.";
    const KEY_EVENT_MESSAGE: &'static str = "Key filters apply only to keydown and keyup events.";

    pub(crate) fn parse(attribute_suffix: &str) -> Result<Self, &'static str> {
        let mut parts = attribute_suffix.split('.');
        let (listen_on, event_type) = Self::target(parts.next().unwrap_or_default());
        Self::refuse_bad_name(event_type)?;
        let options = EventOptions::parse(listen_on, &parts.collect::<Vec<_>>())?;
        Self::refuse_key_on_other_event(event_type, &options)?;
        Ok(Self {
            event_type: String::from(event_type),
            options,
        })
    }

    fn target(name: &str) -> (Option<&'static str>, &str) {
        Self::TARGETS
            .iter()
            .find_map(|target| Self::strip_target(name, target))
            .map_or((None, name), |(target, rest)| (Some(target), rest))
    }

    fn strip_target<'a>(name: &'a str, target: &'static str) -> Option<(&'static str, &'a str)> {
        let rest = name.strip_prefix(target)?.strip_prefix(':')?;
        Some((target, rest))
    }

    fn refuse_bad_name(event_type: &str) -> Result<(), &'static str> {
        let allowed = (1..=Self::MAX_BYTES).contains(&event_type.len())
            && event_type.bytes().all(Self::is_name_byte)
            && !event_type.starts_with('-')
            && !event_type.ends_with('-');
        allowed.then_some(()).ok_or(Self::NAME_MESSAGE)
    }

    const fn is_name_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b':' | b'-')
    }

    fn refuse_key_on_other_event(
        event_type: &str,
        options: &EventOptions,
    ) -> Result<(), &'static str> {
        let refused = options.key.is_some() && !Self::KEY_EVENTS.contains(&event_type);
        (!refused).then_some(()).ok_or(Self::KEY_EVENT_MESSAGE)
    }
}
