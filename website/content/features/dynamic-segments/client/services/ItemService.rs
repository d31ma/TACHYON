#[Service]
#[derive(Default)]
struct ItemService {}

impl ItemService {
    pub fn id_from(&self, path: &str) -> String {
        path.rsplit('/').next().unwrap_or_default().to_string()
    }
}
