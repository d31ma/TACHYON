from models import Guide


@request_route("/guide")
@public
class GuideController:
    @get_route
    def get_guide(self) -> Guide:
        return Guide("Models hold types", 3)
