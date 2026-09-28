use super::call_rule::CallRule;

#[derive(Clone, Copy, Debug)]
pub(super) struct CallMatch<'r> {
    pub(super) rule: &'r CallRule,
    pub(super) start: usize,
    pub(super) name: usize,
    pub(super) group_end: usize,
    pub(super) tail_end: usize,
}
