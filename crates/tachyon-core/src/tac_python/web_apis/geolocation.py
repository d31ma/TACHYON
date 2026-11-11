class Geolocation:
    API = 'geolocation'

    def current_position(self) -> None:
        _TacPythonWebHost.call(self.API, 'currentPosition')
