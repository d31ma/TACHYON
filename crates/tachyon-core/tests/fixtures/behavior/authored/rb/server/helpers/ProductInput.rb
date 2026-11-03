#[Checked]
class ProductInput
  # @return [String]
  #[Required]
  #[NotEmpty]
  #[Length({"min":2,"max":4})]
  attr_reader :name

  # @return [Integer]
  #[Min(0)]
  #[Max(100)]
  attr_reader :price
end
