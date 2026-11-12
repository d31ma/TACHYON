class Passkeys:
    API = 'passkeys'

    def supported(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported') is True

    def check(self) -> None:
        _TacPythonWebHost.call(self.API, 'check')

    def create(self, options_json: str) -> None:
        _TacPythonWebHost.call(self.API, 'create', (options_json,))

    def get(self, options_json: str) -> None:
        _TacPythonWebHost.call(self.API, 'get', (options_json,))

    def offer(self, options_json: str) -> None:
        _TacPythonWebHost.call(self.API, 'offer', (options_json,))

    def cancel(self) -> None:
        _TacPythonWebHost.call(self.API, 'cancel')
