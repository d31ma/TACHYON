final class GuideService {
    var title: String
    var pages: Int

    init() {
        let guide = Guide(title: "Models hold types", pages: 3)
        title = guide.title
        pages = guide.pages
    }
}
