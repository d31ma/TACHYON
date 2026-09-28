import models.Guide

class GuideService {
    var title: String = ""
    var pages: Int = 0

    init {
        val guide = Guide("Models hold types", 3)
        title = guide.title
        pages = guide.pages
    }
}
