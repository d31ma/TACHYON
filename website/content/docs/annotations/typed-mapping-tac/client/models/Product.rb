# @!attribute [r] id
#   @return [Integer]
# @!attribute [r] name
#   @return [String]
# @!attribute [r] detail
#   @return [Detail]
# @!attribute [r] items
#   @return [Array<Detail>]
# @!attribute [r] secret
#   @return [String]
Product = Data.define(:id, :name, :detail, :items, :secret)
