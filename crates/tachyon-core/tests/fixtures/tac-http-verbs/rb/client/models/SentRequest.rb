# @!attribute [r] method
#   @return [String]
# @!attribute [r] path
#   @return [String]
# @!attribute [r] header
#   @return [String]
# @!attribute [r] item
#   @return [String]
SentRequest = Data.define(:method, :path, :header, :item)
