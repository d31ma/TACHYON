use crate::models::Product;

#[RequestRoute("/products")]
#[Public]
struct ProductsController {
    #[Needs]
    #[Named("ProductsService")]
    catalog: std::sync::Arc<ProductsService>,
}
impl ProductsController {
    #[GetRoute]
    fn get_products(&self) -> Vec<Product> {
        self.catalog.list()
    }

    #[GetRoute("/{id}")]
    fn get_product(&self, #[FromPath] id: i64) -> Product {
        self.catalog.find(id)
    }
}
