use super::{EventMethod, EventType};
use crate::Failure;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct EventTimer {
    pub method: String,
    pub every: u64,
}

impl EventTimer {
    pub(crate) const MARKER: &'static str = "Every";
    pub(crate) const MOST_TIMERS: usize = 8;
    const SHORTEST: u64 = 1_000;
    const LONGEST: u64 = 86_400_000;
    const QUOTES: [char; 3] = ['"', '\'', '`'];
    const UNITS: [(&'static str, u64); 4] =
        [("ms", 1), ("s", 1_000), ("m", 60_000), ("h", 3_600_000)];
    const FORM: &'static str = "@Every takes one duration, such as @Every(\"60s\"): a whole number followed by ms, s, m or h, from 1s to 24h";
    const NO_PARAMETERS: &'static str = "An @Every method takes no parameters";
    const TOO_MANY: &'static str = "A Tac service has at most 8 @Every methods";

    pub(crate) fn add(
        path: &Path,
        event_type: &mut EventType,
        method: &EventMethod,
        arguments: &str,
        at: usize,
    ) -> Result<(), Failure> {
        Self::check_room(path, event_type.timers.len(), at)?;
        let timer = Self::read(path, &method.name, method.parameters.len(), arguments, at)?;
        event_type.timers.push(timer);
        Ok(())
    }

    pub(crate) fn checked(
        path: &Path,
        method: &str,
        parameters: usize,
        arguments: &str,
    ) -> Result<Self, Failure> {
        Self::read(path, method, parameters, arguments, 0)
    }

    pub(crate) fn check_count(path: &Path, count: usize) -> Result<(), Failure> {
        Self::check_room(path, count.saturating_sub(1), 0)
    }

    pub(crate) fn duration(arguments: &str) -> Option<u64> {
        let text = Self::unquoted(arguments.trim())?;
        let split = text.find(|letter: char| !letter.is_ascii_digit())?;
        let (count, unit) = text.split_at(split);
        let count = count.parse::<u64>().ok()?;
        let every = count.checked_mul(Self::scale(unit)?)?;
        Self::within_limits(every)
    }

    fn read(
        path: &Path,
        method: &str,
        parameters: usize,
        arguments: &str,
        at: usize,
    ) -> Result<Self, Failure> {
        Self::check_parameters(path, parameters, at)?;
        let every =
            Self::duration(arguments).ok_or_else(|| super::failure(path, at, Self::FORM))?;
        Ok(Self {
            method: method.to_owned(),
            every,
        })
    }

    fn unquoted(text: &str) -> Option<&str> {
        Self::QUOTES
            .iter()
            .find_map(|quote| text.strip_prefix(*quote)?.strip_suffix(*quote))
    }

    fn scale(unit: &str) -> Option<u64> {
        Self::UNITS
            .iter()
            .find(|(name, _)| *name == unit)
            .map(|(_, scale)| *scale)
    }

    fn within_limits(every: u64) -> Option<u64> {
        (Self::SHORTEST..=Self::LONGEST)
            .contains(&every)
            .then_some(every)
    }

    fn check_parameters(path: &Path, parameters: usize, at: usize) -> Result<(), Failure> {
        if parameters == 0 {
            return Ok(());
        }
        Err(super::failure(path, at, Self::NO_PARAMETERS))
    }

    fn check_room(path: &Path, held: usize, at: usize) -> Result<(), Failure> {
        if held < Self::MOST_TIMERS {
            return Ok(());
        }
        Err(super::failure(path, at, Self::TOO_MANY))
    }
}
