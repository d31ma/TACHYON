class Files:
    API = 'files'

    def pick(self, accept: str, multiple: bool) -> None:
        _TacPythonWebHost.call(self.API, 'pick', (accept, multiple))

    def read(self, id: str, offset: int, count: int) -> None:
        _TacPythonWebHost.call(self.API, 'read', (id, offset, count))

    def release(self, id: str) -> None:
        _TacPythonWebHost.call(self.API, 'release', (id,))

    def upload(self, id: str, method: str, url: str, headers: str) -> str:
        return str(_TacPythonWebHost.call(self.API, 'upload', (id, method, url, headers)))

    def cancel(self, upload: str) -> None:
        _TacPythonWebHost.call(self.API, 'cancel', (upload,))
