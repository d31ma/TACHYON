#[Entity({"adapter":"publicMoney"})]
#[Table("money")]
class MoneyEntity
  # @return [Integer]
  #[Id]
  attr_reader :id
end
