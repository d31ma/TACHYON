from models import Guide


@service
class GuideService:
    title: str = ""
    pages: int = 0

    @on_start
    def _start(self) -> None:
        guide = Guide(title="Models hold types", pages=3)
        self.title = guide.title
        self.pages = guide.pages
