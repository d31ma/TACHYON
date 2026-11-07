from typing import Annotated

@service
class AuditService:
    audit: Annotated[AuditDelegate, needs]
    visible: bool = True

    def ready(self) -> str:
        return 'Python ready'

    def snapshot(self) -> list[int]:
        return self.audit.snapshot()

    def hide(self) -> None:
        self.visible = False
