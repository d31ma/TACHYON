#[RequestRoute("/products")]
#[Public]
struct ProductsController {}

impl ProductsController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    fn get_products(&self) -> http::Response<Vec<u8>> {
        CatalogChecks::answer()
    }
}
