from models import InventoryRecord


@service
class InventoryService:
    records: list[InventoryRecord] = []

    @on_event
    def add_record(self, record: InventoryRecord) -> None:
        self.records = [*self.records[-63:], record]
