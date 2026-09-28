from typing import Optional
from urllib.request import Request
from models import Identity


@identity_provider
class AppIdentityProvider:
    _SESSIONS: dict[str, Identity] = {
        "ada-token": Identity("ada", ["Admin"]),
        "bob-token": Identity("bob", []),
    }

    def identify(self, request: Request) -> Optional[Identity]:
        return self._find(str(request.get_header("Authorization")))

    def _find(self, token: str) -> Optional[Identity]:
        return self._SESSIONS.get(token)
