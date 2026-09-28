#[Entity({"adapter":"data"})]
#[Table("products")]
class ProductEntity
  # @return [Integer]
  #[Id]
  attr_reader :id
end
