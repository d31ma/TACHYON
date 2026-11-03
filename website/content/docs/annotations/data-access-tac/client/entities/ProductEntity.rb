#[Entity({"adapter":"publicCatalog"})]
#[Table("products")]
class ProductEntity
  # @return [Integer]
  #[Id]
  attr_reader :id

  # @return [String]
  attr_reader :status
end
