using Models;

[RequestRoute("/guide")]
[Public]
class GuideController {
    [GetRoute]
    public Guide GetGuide() {
        return new Guide("Models hold types", 3);
    }
}
