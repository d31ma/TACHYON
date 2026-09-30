#[Entity({"adapter":"billing"})]
#[Table("invoices")]
class BillingEntity
  # @return [Integer]
  #[Id]
  attr_reader :id
end
