pub(crate) struct TakenOn<'a> {
    pub(crate) service: String,
    pub(crate) service_source: &'a str,
    pub(crate) methods: Vec<(String, usize)>,
}
