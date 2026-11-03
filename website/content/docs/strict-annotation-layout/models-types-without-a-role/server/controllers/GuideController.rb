#[RequestRoute("/guide")]
#[Public]
class GuideController
  # @return [Guide]
  #[GetRoute]
  def get_guide
    Guide.new(title: "Models hold types", pages: 3)
  end
end
