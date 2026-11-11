import builtins as _tac_web_builtins


class SecureRandom:
    API = 'secureRandom'

    def bytes(self, count: int) -> _tac_web_builtins.bytes:
        return _tac_web_builtins.bytes(_TacPythonWebHost.call(self.API, 'bytes', (count,)))

    def uuid(self) -> str:
        return str(_TacPythonWebHost.call(self.API, 'uuid'))
