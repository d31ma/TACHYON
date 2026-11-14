class CodeScanner:
    API = 'codeScanner'

    def supported(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported') is True

    def scan(self) -> None:
        _TacPythonWebHost.call(self.API, 'scan')

    def cancel(self) -> None:
        _TacPythonWebHost.call(self.API, 'cancel')
