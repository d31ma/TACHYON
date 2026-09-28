import models.Guide

@RequestRoute("/guide")
@Public
class GuideController {
    @GetRoute
    Guide getGuide() {
        return new Guide("Models hold types", 3)
    }
}
