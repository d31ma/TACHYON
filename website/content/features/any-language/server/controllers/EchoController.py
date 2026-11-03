from typing import Annotated
from models import Message


@request_route("/echo")
@public
class EchoController:
    @post_route
    def post_echo(self, message: Annotated[Message, from_body]) -> Message:
        return message
