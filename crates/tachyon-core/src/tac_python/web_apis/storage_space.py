class StorageSpace:
    API = 'storageSpace'

    def estimate(self) -> None:
        _TacPythonWebHost.call(self.API, 'estimate')

    def persist(self) -> None:
        _TacPythonWebHost.call(self.API, 'persist')
