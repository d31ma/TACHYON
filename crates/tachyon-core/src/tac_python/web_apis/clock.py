class Clock:
    API = 'clock'

    def now_ms(self) -> float:
        return float(_TacPythonWebHost.call(self.API, 'now'))

    def monotonic_ms(self) -> float:
        return float(_TacPythonWebHost.call(self.API, 'monotonic'))

    def time_zone(self) -> str:
        return str(_TacPythonWebHost.call(self.API, 'timeZone'))
