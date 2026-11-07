use super::{Choice, ChoiceError};
use crate::Failure;

pub struct ChoiceList;

impl ChoiceList {
    pub fn parse<T: Choice>(text: &str, topic: &str) -> Result<Vec<T>, Failure> {
        let chosen = Self::parts(text)
            .into_iter()
            .map(|part| Self::one::<T>(part, topic))
            .collect::<Result<Vec<T>, Failure>>()?;
        Ok(Self::in_standard_order(&chosen))
    }

    pub fn one<T: Choice>(text: &str, topic: &str) -> Result<T, Failure> {
        T::named(text).ok_or_else(|| ChoiceError::unknown::<T>(text.trim(), topic))
    }

    fn parts(text: &str) -> Vec<&str> {
        text.split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect()
    }

    fn in_standard_order<T: Choice>(chosen: &[T]) -> Vec<T> {
        T::ALL
            .iter()
            .copied()
            .filter(|choice| chosen.contains(choice))
            .collect()
    }
}
