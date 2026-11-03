pub(crate) struct TypeName<'a> {
    pub(crate) access: &'static str,
    pub(crate) name: &'a str,
    pub(crate) generics: String,
    pub(crate) rest: &'a str,
}
