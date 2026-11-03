from typing import Protocol


@http_client
class ApplicationClient(Protocol):
    @get_request("/manifest.webmanifest")
    async def load(self) -> dict[str, object]: ...
