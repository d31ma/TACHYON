import java.util.ArrayList;
import java.util.List;
import models.InventoryRecord;

@Service
class InventoryService {
    List<InventoryRecord> records = new ArrayList<>();

    @OnEvent
    void addRecord(InventoryRecord record) {
        int older = Math.max(0, records.size() - 63);
        List<InventoryRecord> kept = new ArrayList<>(records.subList(older, records.size()));
        kept.add(record);
        records = kept;
    }
}
