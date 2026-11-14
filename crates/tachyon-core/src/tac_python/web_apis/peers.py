class Peers:
    API = 'peers'

    def supported(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported') is True

    def open(self, username: str, credential: str) -> str:
        return str(_TacPythonWebHost.call(self.API, 'open', (username, credential)))

    def offer(self, connection: str) -> None:
        _TacPythonWebHost.call(self.API, 'offer', (connection,))

    def answer(self, connection: str, offer_sdp: str) -> None:
        _TacPythonWebHost.call(self.API, 'answer', (connection, offer_sdp))

    def accept(self, connection: str, answer_sdp: str) -> None:
        _TacPythonWebHost.call(self.API, 'accept', (connection, answer_sdp))

    def add_candidate(self, connection: str, candidate_json: str) -> None:
        _TacPythonWebHost.call(self.API, 'addCandidate', (connection, candidate_json))

    def restart(self, connection: str) -> None:
        _TacPythonWebHost.call(self.API, 'restart', (connection,))

    def open_channel(self, connection: str, label: str, ordered: bool) -> str:
        return str(_TacPythonWebHost.call(self.API, 'openChannel', (connection, label, ordered)))

    def send(self, connection: str, channel: str, data: str) -> bool:
        return _TacPythonWebHost.call(self.API, 'send', (connection, channel, data)) is True

    def close_channel(self, connection: str, channel: str) -> None:
        _TacPythonWebHost.call(self.API, 'closeChannel', (connection, channel))

    def close(self, connection: str) -> None:
        _TacPythonWebHost.call(self.API, 'close', (connection,))
