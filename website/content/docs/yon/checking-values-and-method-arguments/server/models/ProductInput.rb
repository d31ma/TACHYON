#[Checked]
class ProductInput
  # @return [String]
  #[NotEmpty]
  #[Length({"min":1,"max":80})]
  attr_reader :name

  # @return [Integer]
  #[Required]
  #[Min(1)]
  #[Max(9)]
  attr_reader :level
end
