from models import ChatMessage


@socket_route("/chat")
@public
class ChatController:
    @on_open
    def on_open(self, session: object) -> None:
        session.send(ChatMessage("Welcome to the room"))

    @on_message
    def on_message(self, session: object, message: ChatMessage) -> None:
        self._send_to_everyone(session.open_sessions(), message)

    def _send_to_everyone(self, peers: list[object], message: ChatMessage) -> None:
        for peer in peers:
            peer.send(message)
