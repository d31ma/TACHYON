use crate::models::InventoryRecord;

#[RequestRoute("/inventory")]
#[Public]
struct InventoryController {
    #[Needs]
    application_event_publisher: std::sync::Arc<ApplicationEventPublisher>,
}
impl InventoryController {
    #[PostRoute]
    fn post_record(&self, #[FromBody] record: InventoryRecord) -> InventoryRecord {
        self.application_event_publisher.publish_event(record.clone(), false);
        record
    }
}
