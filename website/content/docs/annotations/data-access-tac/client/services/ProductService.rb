#[Service]
class ProductService
  #[Needs]
  # @return [ProductStore]
  attr_reader :product_store

  # @return [String]
  attr_reader :summary

  def initialize
    @summary = "Not loaded."
  end

  # @return [void]
  def load
    rows = product_store.excluding("retired", { "size" => 5 })
    active_count = product_store.count_by_status("active")
    @summary = "#{rows.length} of #{active_count} active"
  end
end
