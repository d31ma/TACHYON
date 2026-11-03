pub struct Endpoint {
    #[Setting("${yon.active.profile:development}")]
    pub profile: String,
}
