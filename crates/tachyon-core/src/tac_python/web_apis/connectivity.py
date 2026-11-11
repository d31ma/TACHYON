class Connectivity:
    API = 'connectivity'

    def online(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'online') is True
