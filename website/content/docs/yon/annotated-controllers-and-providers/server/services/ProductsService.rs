use crate::models::Product;

#[Service]
#[Lifetime("transient")]
struct ProductsService {}
impl ProductsService {
    fn new() -> Self { Self {} }
    fn list(&self) -> Vec<Product> { vec![self.find(1)] }
    fn find(&self, id: i64) -> Product { Product { id, name: String::from("Anvil") } }
}
