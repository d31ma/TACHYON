class GuideService
  # @return [String]
  attr_reader :title
  # @return [Integer]
  attr_reader :pages

  # @return [void]
  #[OnStart]
  def start
    guide = Guide.new(title: "Models hold types", pages: 3)
    @title = guide.title
    @pages = guide.pages
  end
end
