@service
class ActivityService:
    state: str = 'foreground'

    @on_event({"replay":true})
    def application_state_changed(self, changed: ApplicationStateChanged) -> None:
        self.state = changed.state
