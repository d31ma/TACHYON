#[Service]
#[NeedsAll]
class DocumentsService
  # @return [Array<String>]
  attr_reader :order

  # @return [Integer]
  def guides
    @order.length
  end
end
