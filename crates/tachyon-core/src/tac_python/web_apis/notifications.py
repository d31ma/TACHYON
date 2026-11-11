class Notifications:
    API = 'notifications'

    def permission(self) -> str:
        return str(_TacPythonWebHost.call(self.API, 'permission'))

    def request_permission(self) -> None:
        _TacPythonWebHost.call(self.API, 'requestPermission')

    def show(self, title: str, body: str) -> None:
        _TacPythonWebHost.call(self.API, 'show', (title, body))
