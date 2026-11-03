#[Service]
pub struct CatalogService {
    pub title: &'static str,
}

impl CatalogService {
    pub fn new() -> Self {
        Self { title: "Catalog" }
    }
}
