import models.Guide

@RequestRoute("/guide")
@Public
class GuideController {
    @GetRoute
    fun getGuide(): Guide = Guide("Models hold types", 3)
}
