#[allow(dead_code)]
struct Endpoint {
    #[Setting("${yon.active.profile:development}")]
    profile: String,
}
