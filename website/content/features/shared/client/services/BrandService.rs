#[Service]
pub struct BrandService {
    pub logo: &'static str,
}

impl BrandService {
    pub fn new() -> Self {
        Self { logo: "/shared/assets/logo.svg" }
    }
}
