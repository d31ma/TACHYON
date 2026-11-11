class PageVisibility:
    API = 'pageVisibility'

    def visible(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'visible') is True
