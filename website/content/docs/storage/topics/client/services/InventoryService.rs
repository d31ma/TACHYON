use crate::models::InventoryRecord;

#[Service]
#[derive(Default)]
pub struct InventoryService {
    pub records: Vec<InventoryRecord>,
}

impl InventoryService {
    #[OnEvent]
    pub fn add_record(&mut self, record: InventoryRecord) {
        let older = self.records.len().saturating_sub(63);
        self.records.drain(..older);
        self.records.push(record);
    }
}
