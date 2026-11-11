class _TacPythonBrowserClients:
    types = {}

    @classmethod
    def register(cls, *clients: type) -> None:
        cls.types.update({client.__name__: client for client in clients})

    @classmethod
    def client(cls, name: str) -> object:
        return cls.types[name]()
