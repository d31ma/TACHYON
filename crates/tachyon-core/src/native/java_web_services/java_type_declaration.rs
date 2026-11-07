pub(super) struct JavaTypeDeclaration {
    pub(super) keyword_at: usize,
    pub(super) keyword: &'static str,
    pub(super) name: String,
    pub(super) name_end: usize,
}
