pub(crate) struct WindowHashes;

impl WindowHashes {
    const BASE: u64 = 1_000_003;

    pub(crate) fn of(ids: &[u64], width: usize) -> Vec<u64> {
        let count = (ids.len() + 1).saturating_sub(width);
        let first = Self::first(ids, width);
        let power = Self::power(width);
        let rolled = (1..count).scan(first, |hash, at| {
            *hash = Self::roll(*hash, ids[at - 1], ids[at + width - 1], power);
            Some(*hash)
        });
        std::iter::once(first).chain(rolled).take(count).collect()
    }

    fn first(ids: &[u64], width: usize) -> u64 {
        ids.iter().take(width).fold(0, |hash, id| {
            hash.wrapping_mul(Self::BASE).wrapping_add(*id)
        })
    }

    fn power(width: usize) -> u64 {
        (1..width).fold(1, |power, _| power.wrapping_mul(Self::BASE))
    }

    fn roll(hash: u64, old: u64, new: u64, power: u64) -> u64 {
        hash.wrapping_sub(old.wrapping_mul(power))
            .wrapping_mul(Self::BASE)
            .wrapping_add(new)
    }
}
