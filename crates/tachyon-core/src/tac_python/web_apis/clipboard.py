class Clipboard:
    API = 'clipboard'

    def write_text(self, text: str) -> None:
        _TacPythonWebHost.call(self.API, 'writeText', (text,))

    def read_text(self) -> None:
        _TacPythonWebHost.call(self.API, 'readText')
