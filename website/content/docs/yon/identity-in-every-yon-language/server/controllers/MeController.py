from typing import Annotated
from models import Identity


@request_route("/me")
@signed_in
class MeController:
    @get_route
    def get_me(self, caller: Annotated[Identity, from_identity]) -> str:
        return caller.user
