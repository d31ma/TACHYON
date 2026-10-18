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
    fn post_product(&self, #[FromBody] product: Product) -> Result<Product, BehaviorError> {
        let input = BehaviorValue::object([
            ("name", BehaviorValue::from(product.name.as_str())),
            ("sku", BehaviorValue::from(product.sku.as_str())),
            ("level", BehaviorValue::Number(product.level as f64)),
        ]);
        self.product_service.accept(input)?;
        Ok(product)
    }
}
