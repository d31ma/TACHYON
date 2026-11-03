use crate::models::SectionChosen;

#[Service]
#[Lifetime({"value":"application"})]
struct SectionMenuService {
    page_events: CurrentPageEvents,
    refreshes: std::sync::atomic::AtomicI64,
}

impl SectionMenuService {
    fn new(page_events: CurrentPageEvents) -> Self {
        Self { page_events, refreshes: std::sync::atomic::AtomicI64::new(0) }
    }

    fn show_running(&self) {
        self.page_events.publish_event(SectionChosen { section: String::from("running") });
    }

    fn show_activity(&self) {
        self.page_events.publish_event(SectionChosen { section: String::from("activity") });
    }

    fn refresh(&self) {
        self.refreshes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
