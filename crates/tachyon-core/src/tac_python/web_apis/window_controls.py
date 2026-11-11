class WindowControls:
    API = 'windowControls'

    def granted(self, control: str) -> bool:
        return _TacPythonWebHost.call(self.API, 'granted', (control,)) is True

    def minimize(self) -> None:
        _TacPythonWebHost.call(self.API, 'minimize')

    def maximize(self) -> None:
        _TacPythonWebHost.call(self.API, 'maximize')

    def fullscreen(self) -> None:
        _TacPythonWebHost.call(self.API, 'fullscreen')

    def close(self) -> None:
        _TacPythonWebHost.call(self.API, 'close')

    def resize(self, width: int, height: int) -> None:
        _TacPythonWebHost.call(self.API, 'resize', (width, height))

    def set_title(self, text: str) -> None:
        _TacPythonWebHost.call(self.API, 'setTitle', (text,))
