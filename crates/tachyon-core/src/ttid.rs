use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

struct TtidFormat;

impl TtidFormat {
    const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const TICKS_PER_MILLISECOND: u128 = 10_000;
}

#[must_use]
pub fn generate() -> String {
    static LAST_ISSUED: AtomicU64 = AtomicU64::new(0);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() / 100);
    let now = u64::try_from(now).unwrap_or(u64::MAX);

    let mut previous = LAST_ISSUED.load(Ordering::Relaxed);
    let issued = loop {
        let candidate = if now > previous { now } else { previous + 1 };
        let exchange = LAST_ISSUED.compare_exchange_weak(
            previous,
            candidate,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        match exchange {
            Ok(_) => break candidate,
            Err(observed) => previous = observed,
        }
    };
    encode(issued)
}

fn encode(mut value: u64) -> String {
    if value == 0 {
        return String::from("0");
    }
    let mut digits = Vec::with_capacity(13);
    while value > 0 {
        let index = usize::try_from(value % 36).unwrap_or(0);
        digits.push(TtidFormat::DIGITS[index]);
        value /= 36;
    }
    digits.reverse();
    String::from_utf8(digits).unwrap_or_default()
}

#[must_use]
pub fn created_at_milliseconds(id: &str) -> Option<u128> {
    let mut ticks: u128 = 0;
    for byte in id.bytes() {
        let digit = TtidFormat::DIGITS
            .iter()
            .position(|candidate| *candidate == byte)?;
        ticks = ticks.checked_mul(36)?.checked_add(digit as u128)?;
    }
    Some(ticks / TtidFormat::TICKS_PER_MILLISECOND)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{created_at_milliseconds, generate};
    use std::collections::BTreeSet;

    #[test]
    fn identifiers_match_the_ttid_shape() {
        let id = generate();
        assert_eq!(id.len(), 11, "{id}");
        assert!(
            id.bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()),
            "{id}"
        );
    }

    #[test]
    fn identifiers_are_valid_handler_protocol_identifiers() {
        let id = generate();
        assert!((1..=128).contains(&id.len()));
        assert!(
            id.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        );
    }

    #[test]
    fn identifiers_are_unique_and_strictly_increasing() {
        let issued: Vec<String> = (0..10_000).map(|_| generate()).collect();
        let unique: BTreeSet<&String> = issued.iter().collect();
        assert_eq!(unique.len(), issued.len(), "identifiers collided");

        let mut sorted = issued.clone();
        sorted.sort();
        assert_eq!(sorted, issued, "identifiers are not lexically ordered");
    }

    #[test]
    fn creation_time_decodes_to_the_present() {
        let id = generate();
        let decoded = created_at_milliseconds(&id).expect("decodable");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_millis();
        assert!(decoded <= now + 1_000, "{decoded} is ahead of {now}");
        assert!(decoded + 60_000 >= now, "{decoded} is far behind {now}");
        assert!(decoded > 1_577_836_800_000, "{decoded}");
    }

    #[test]
    fn malformed_identifiers_do_not_decode() {
        assert!(created_at_milliseconds("lowercase").is_none());
        assert!(created_at_milliseconds("has-hyphen").is_none());
        assert!(created_at_milliseconds("").is_some());
    }
}
