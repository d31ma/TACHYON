use crate::models::CatalogCount;

#[derive(Default)]
struct CatalogService {
    count: i64,
    application_event_publisher: ApplicationEventPublisher,
}
impl CatalogService {
    #[OnStart]
    fn initialize(&mut self) { self.announce(); }

    fn announce(&self) {
        self.application_event_publisher.publish_event(CatalogCount { count: 5 }, false);
    }

    #[OnEvent]
    fn changed(&mut self, value: CatalogCount) { self.count = value.count; }

    #[OnStop]
    fn cleanup(&mut self) { self.count = 0; }
}
