use crate::models::Product;
use std::sync::Arc;

#[RequestRoute("/products")]
#[Public]
struct ProductsController {
    #[Needs]
    product_service: Arc<ProductService>,
}
impl ProductsController {
    #[PostRoute]
    fn post_product(&self, #[FromBody] product: Product) -> http::Response<Vec<u8>> {
        let input = BehaviorValue::object([
            ("name", BehaviorValue::from(product.name)),
            ("sku", BehaviorValue::from(product.sku)),
            ("level", BehaviorValue::Number(product.level as f64)),
        ]);
        Self::answer(Self::checked_text(&self.product_service, input))
    }

    fn checked_text(products: &ProductService, input: BehaviorValue) -> Option<String> {
        let checked = products.accept(input).ok()?;
        checked.to_json(65_536).ok()
    }

    fn answer(text: Option<String>) -> http::Response<Vec<u8>> {
        match text {
            Some(text) => Self::response(200, text),
            None => Self::response(400, String::from("{\"error\":\"validation\"}")),
        }
    }

    fn response(status: u16, body: String) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(body.into_bytes())
            .unwrap_or_default()
    }
}
