use crate::models::Product;

#[RequestRoute("/products")]
#[Public]
struct ProductsController {}
impl ProductsController {
    fn new() -> Self { Self {} }

    #[GetRoute]
    fn get_products(&self) -> Vec<Product> {
        vec![Product { name: String::from("anvil") }]
    }

    #[PostRoute]
    #[ResponseStatus(201)]
    fn post_product(&self, #[FromBody] product: Product) -> Product {
        product
    }
}
