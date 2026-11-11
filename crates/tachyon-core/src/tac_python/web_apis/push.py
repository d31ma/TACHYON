class Push:
    API = 'push'

    def supported(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported') is True

    def subscribe(self, application_server_key: str) -> None:
        _TacPythonWebHost.call(self.API, 'subscribe', (application_server_key,))

    def subscription(self) -> None:
        _TacPythonWebHost.call(self.API, 'subscription')

    def unsubscribe(self) -> None:
        _TacPythonWebHost.call(self.API, 'unsubscribe')
