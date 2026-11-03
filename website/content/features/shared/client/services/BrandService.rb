#[Service]
class BrandService
  # @return [String]
  attr_reader :logo

  def initialize
    @logo = "/shared/assets/logo.svg"
  end
end
