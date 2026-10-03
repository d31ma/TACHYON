#[Service]
struct ProductService { summary: String }
impl ProductService {
    fn new() -> Self { Self { summary: String::from("Not loaded.") } }
}
