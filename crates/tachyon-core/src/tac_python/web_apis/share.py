class Share:
    API = 'share'

    def open(self, title: str, text: str, url: str) -> None:
        _TacPythonWebHost.call(self.API, 'open', (title, text, url))
