#[yon::service]
struct ProductService {}
impl ProductService {
    fn new() -> Self { Self {} }

    fn accept(&self, #[Check(ProductInput)] input: BehaviorValue) -> BehaviorResult<BehaviorValue> {
        Ok(input)
    }
}
