use std::ops::Sub;

pub struct NativeCounterUtils;

impl NativeCounterUtils {
    const ZERO: u8 = 0;
    const CHECK_COUNT: usize = 4;
    pub fn delta<T: Copy + PartialOrd + Sub<Output = T> + From<u8>>(
        current: Option<T>,
        previous: Option<T>,
        duration: Option<f64>,
        valid: bool,
    ) -> Option<T> {
        let values: Option<(T, T)> = current.zip(previous);
        let ready: bool = Self::ready(values, duration, valid);
        Self::selected(values, ready)
    }

    fn ready<T: Copy + PartialOrd + From<u8>>(
        values: Option<(T, T)>,
        duration: Option<f64>,
        valid: bool,
    ) -> bool {
        let checks: Option<[bool; Self::CHECK_COUNT]> =
            values.map(|(current, previous)| Self::checks(current, previous, duration, valid));
        checks.map(Self::all).unwrap_or(false)
    }

    fn checks<T: Copy + PartialOrd + From<u8>>(
        current: T,
        previous: T,
        duration: Option<f64>,
        valid: bool,
    ) -> [bool; Self::CHECK_COUNT] {
        [
            valid,
            duration.is_some(),
            current >= previous,
            previous >= T::from(Self::ZERO),
        ]
    }

    fn all(checks: [bool; Self::CHECK_COUNT]) -> bool {
        checks.into_iter().all(std::convert::identity)
    }

    fn selected<T: Copy + Sub<Output = T>>(values: Option<(T, T)>, ready: bool) -> Option<T> {
        if !ready {
            return None;
        }
        values.map(|(current, previous)| current - previous)
    }
}
