#[Entity({"adapter":"ledger"})]
#[Table("entries")]
class EntryEntity
  # @return [Integer]
  #[Id]
  attr_reader :id
end
