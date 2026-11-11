# @!attribute [r] title
#   @return [String]
# @!attribute [r] status
#   @return [Integer]
# @!attribute [r] errors
#   @return [Array<ProblemEntry>]
Problem = Data.define(:title, :status, :errors)
