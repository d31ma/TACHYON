use crate::models::*;
#[Converter]
trait ProductConverter {
    #[Convert({"fields":[{"target":"kind","constant":"product"}]})]
    fn map(&self, product: Product) -> BehaviorResult<Summary>;
}
