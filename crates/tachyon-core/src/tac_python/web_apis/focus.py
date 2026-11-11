class Focus:
    API = 'focus'

    def move(self, target: str) -> None:
        _TacPythonWebHost.call(self.API, 'move', (target,))

    def trap(self, container: str) -> None:
        _TacPythonWebHost.call(self.API, 'trap', (container,))

    def release(self) -> None:
        _TacPythonWebHost.call(self.API, 'release')
