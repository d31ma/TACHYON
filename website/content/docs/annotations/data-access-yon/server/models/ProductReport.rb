# @!attribute [r] active
#   @return [Array<Hash>]
# @!attribute [r] count
#   @return [Integer]
# @!attribute [r] others
#   @return [Array<Hash>]
# @!attribute [r] first
#   @return [Hash]
ProductReport = Data.define(:active, :count, :others, :first)
