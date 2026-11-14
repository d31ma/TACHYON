from typing import Optional
from urllib.request import Request
from models import Identity


@identity_provider
class AppIdentityProvider:
    _SESSIONS: dict[str, Identity] = {
        "ada-token": Identity("ada", ["Admin"]),
        "bob-token": Identity("bob", []),
    }
    _BROKEN_TOKEN: str = "broken-token"

    def identify(self, request: Request) -> Optional[Identity]:
        token = str(request.get_header("Authorization"))
        self._refuse_broken(token)
        return self._find(token)

    def _refuse_broken(self, token: str) -> None:
        if token == self._BROKEN_TOKEN:
            raise ConnectionError("the session store is down")

    def _find(self, token: str) -> Optional[Identity]:
        return self._SESSIONS.get(token)
