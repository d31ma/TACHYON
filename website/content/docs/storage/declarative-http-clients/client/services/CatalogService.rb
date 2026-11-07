#[Service]
class CatalogService
  #[Needs]
  # @return [CatalogClient]
  attr_reader :catalog_client

  # @return [Array<String>]
  attr_reader :products
  # @return [String]
  attr_reader :error

  def initialize
    @products = []
    @error = ""
  end

  # @return [void]
  def load
    show(fetch)
  end

  private

  # @return [Array<String>, nil]
  def fetch
    catalog_client.all
  rescue StandardError
    nil
  end

  # @param answer [Array<String>, nil]
  # @return [void]
  def show(answer)
    return show_failure unless answer.is_a?(Array)

    @products = answer
    @error = ""
  end

  # @return [void]
  def show_failure
    @error = "Products could not be loaded. Please retry."
  end
end
