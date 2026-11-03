from typing import Annotated, Optional
from models import ProbeReady


@request_route("/probe")
@public
class ProbeController:
    probe_service: Annotated[ProbeService, needs]
    publish_service: Annotated[PublishService, needs]
    @get_route
    def get_probe(self) -> Optional[ProbeReady]:
        return self.probe_service.last
