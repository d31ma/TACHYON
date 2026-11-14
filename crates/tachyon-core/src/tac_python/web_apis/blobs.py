class Blobs:
    API = 'blobs'

    def save_held(self, file_id: str, key: str) -> None:
        _TacPythonWebHost.call(self.API, 'saveHeld', (file_id, key))

    def download(self, url: str, key: str, headers: str) -> None:
        _TacPythonWebHost.call(self.API, 'download', (url, key, headers))

    def list(self, prefix: str) -> None:
        _TacPythonWebHost.call(self.API, 'list', (prefix,))

    def read(self, key: str, offset: int, count: int) -> None:
        _TacPythonWebHost.call(self.API, 'read', (key, offset, count))

    def remove(self, key: str) -> None:
        _TacPythonWebHost.call(self.API, 'remove', (key,))

    def url(self, key: str) -> str:
        return str(_TacPythonWebHost.call(self.API, 'url', (key,)))

    def open(self, key: str) -> None:
        _TacPythonWebHost.call(self.API, 'open', (key,))

    def close(self, key: str) -> None:
        _TacPythonWebHost.call(self.API, 'close', (key,))

    def upload(self, key: str, method: str, url: str, headers: str) -> str:
        return str(_TacPythonWebHost.call(self.API, 'upload', (key, method, url, headers)))

    def cancel(self, upload: str) -> None:
        _TacPythonWebHost.call(self.API, 'cancel', (upload,))
