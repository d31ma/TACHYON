# @!attribute [r] id
#   @return [Integer]
# @!attribute [r] name
#   @return [String]
# @!attribute [r] detail
#   @return [ShortDetail]
# @!attribute [r] items
#   @return [Array<ShortDetail>]
# @!attribute [r] kind
#   @return [String]
Summary = Data.define(:id, :name, :detail, :items, :kind)
