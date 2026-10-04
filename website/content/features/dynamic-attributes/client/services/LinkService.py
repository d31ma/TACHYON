@service
class LinkService:
    topic: Topic
    state: str
    message: str

    def __init__(self) -> None:
        self.topic = Topic(slug="routing", title="Routing")
        self.state = "ready"
        self.message = "Everything is up to date."
