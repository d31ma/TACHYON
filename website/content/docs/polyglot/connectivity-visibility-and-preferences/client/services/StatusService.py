@service
class StatusService:
    online: bool = True
    scheme: str = 'light'

    @on_event({"replay":true})
    def connectivity_changed(self, changed: ConnectivityChanged) -> None:
        self.online = changed.online

    @on_event({"replay":true})
    def preferences_changed(self, changed: PreferencesChanged) -> None:
        self.scheme = changed.scheme
