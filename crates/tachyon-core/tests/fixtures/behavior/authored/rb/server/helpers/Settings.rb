#[Settings({"prefix":"YON_CATALOG_","defaults":{"size":20,"enabled":false}})]
class Settings
  # @return [Integer]
  #[Min(1)]
  #[Max(100)]
  attr_reader :size

  # @return [Boolean]
  attr_reader :enabled
end
