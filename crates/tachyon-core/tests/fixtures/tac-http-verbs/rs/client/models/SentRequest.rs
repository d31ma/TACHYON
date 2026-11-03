#[derive(Clone, Debug, Default)]
pub struct SentRequest {
    pub method: String,
    pub path: String,
    pub header: String,
    pub item: String,
}
