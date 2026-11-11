class Preferences:
    API = 'preferences'

    def scheme(self) -> str:
        return self._text('scheme')

    def motion(self) -> str:
        return self._text('motion')

    def language(self) -> str:
        return self._text('language')

    def _text(self, call: str) -> str:
        return str(_TacPythonWebHost.call(self.API, call))
