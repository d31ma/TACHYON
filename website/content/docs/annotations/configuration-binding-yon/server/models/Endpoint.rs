#[allow(dead_code)]
pub struct Endpoint {
    #[Setting("${yon.active.profile:development}")]
    profile: String,
}
