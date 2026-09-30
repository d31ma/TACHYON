#[RequestRoute("/products")]
#[Public]
struct ProductsController {}
impl ProductsController {
    fn new() -> Self { Self {} }

    #[GetRoute]
    fn get_products(&self) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body(b"<main><h1>Products</h1><p>{not-a-template}</p><if>literal</if></main>".to_vec())
            .unwrap_or_default()
    }
}
