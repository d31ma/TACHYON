class Locks:
    API = 'locks'

    def request(self, name: str) -> None:
        _TacPythonWebHost.call(self.API, 'request', (name,))

    def release(self, name: str) -> None:
        _TacPythonWebHost.call(self.API, 'release', (name,))
