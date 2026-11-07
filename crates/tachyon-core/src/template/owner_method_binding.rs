pub(super) struct OwnerMethodBinding;

impl OwnerMethodBinding {
    const BINDERS: [&'static str; 3] = [".bind(", ".call(", ".apply("];
    const OWNER: &'static str = "this";
    const MEMBER_ACCESS: [char; 2] = ['.', '['];
    const CHAIN_MARKS: &'static str = "_$.)]";
    const CLOSERS: &'static str = ")]";
    const OPENERS: &'static str = "([";

    pub(super) fn binds_owner(code: &str, offset: usize) -> bool {
        let before = code[..offset].trim_end();
        Self::BINDERS
            .iter()
            .find_map(|binder| before.strip_suffix(binder))
            .is_some_and(|callee| Self::rooted_at_owner(callee.trim_end()))
    }

    fn rooted_at_owner(callee: &str) -> bool {
        let chain = &callee[Self::chain_start(callee)..];
        chain
            .strip_prefix(Self::OWNER)
            .is_some_and(|member| member.starts_with(Self::MEMBER_ACCESS))
    }

    fn chain_start(callee: &str) -> usize {
        let mut depth = 0_isize;
        let mut start = 0;
        for (index, character) in callee.char_indices().rev() {
            let stop = usize::from(Self::stops(depth, character));
            start = start.max(stop * (index + character.len_utf8()));
            depth += Self::nesting(character);
        }
        start
    }

    fn stops(depth: isize, character: char) -> bool {
        [depth == 0, Self::chained(character)] == [true, false]
    }

    fn chained(character: char) -> bool {
        [
            character.is_alphanumeric(),
            Self::CHAIN_MARKS.contains(character),
        ]
        .contains(&true)
    }

    fn nesting(character: char) -> isize {
        isize::from(Self::CLOSERS.contains(character))
            - isize::from(Self::OPENERS.contains(character))
    }
}
