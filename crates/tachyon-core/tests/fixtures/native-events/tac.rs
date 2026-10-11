use crate::models::{Cycle, Echo, Isolation, Ordered, Other, Pulse, Relay};

#[derive(Default)]
#[Lifetime({"value":"prototype"})]
struct OwnerProbeService {
    count: i64,
    log: String,
    deaths: i64,
    application_event_publisher: ApplicationEventPublisher,
}
impl OwnerProbeService {
    #[OnStart]
    fn start(&mut self) {
        self.count = 1;
        self.deaths = tac_session("eventDeaths", "0").parse().unwrap_or(0);
        self.send();
    }
    #[OnStop]
    fn stop(&mut self) {
        tac_set_session("eventDeaths", &(self.deaths + 1).to_string());
    }
    fn send(&self) -> i64 {
        self.application_event_publisher.publish_event(Pulse { amount: 5 }, true);
        5
    }
    #[OnEvent]
    fn receive(&mut self, value: Pulse) {
        self.count += value.amount;
        self.relay();
    }
    fn relay(&self) -> i64 {
        self.application_event_publisher.publish_event(Relay { amount: 3 }, false);
        3
    }
    #[OnEvent]
    fn receive_relay(&mut self, value: Relay) {
        self.count += value.amount;
    }
    fn cycle(&self) -> i64 {
        self.application_event_publisher.publish_event(Cycle { amount: 1 }, false);
        1
    }
    #[OnEvent]
    fn receive_cycle(&mut self, value: Cycle) {
        self.count += value.amount;
        self.cycle();
    }
    fn isolated(&self) -> TacValue {
        self.application_event_publisher.publish_event(Isolation { value: 1 }, false);
        TacValue::Object(vec![("value".into(), TacValue::Int(1))])
    }
    #[OnEvent]
    fn a_mutation(&self, value: Isolation) {
        let mut copy = value;
        copy.value = 99;
    }
    #[OnEvent]
    fn b_original(&mut self, value: Isolation) {
        self.count += value.value;
    }
    fn publish_ordered(&self) -> i64 {
        let ordered = Ordered { status: String::from("paid"), total: 12 };
        self.application_event_publisher.publish_event(ordered, false);
        1
    }
    fn publish_other(&self) -> i64 {
        let other = Other { status: String::from("paid"), total: 12 };
        self.application_event_publisher.publish_event(other, false);
        1
    }
    #[Order(-1)]
    #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    fn ordinal(&mut self, value: Ordered) -> Echo {
        self.log.push('1');
        Echo { amount: 7 }
    }
    #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
    fn other_ordinal(&mut self, value: Other) -> Echo {
        self.log.push('1');
        Echo { amount: 7 }
    }
    #[OnEvent]
    fn second(&mut self, value: Ordered) {
        self.log.push('2');
    }
    #[OnEvent]
    fn failing(&mut self, value: Ordered) -> Result<TacValue, &'static str> {
        Err("fixture")
    }
    #[OnEvent({"condition":"value.total >= 100"})]
    fn skipped(&mut self, value: Ordered) {
        self.log.push('S');
    }
    #[Order(1)]
    #[InBackground]
    #[OnEvent]
    fn deferred_listener(&mut self, value: Ordered) -> Echo {
        self.log.push('A');
        Echo { amount: 1000 }
    }
    #[Order(2)]
    #[OnEvent]
    fn third(&mut self, value: Ordered) {
        self.log.push('3');
    }
    #[OnEvent]
    fn on_echo(&mut self, value: Echo) {
        self.log.push('E');
        self.count += value.amount;
    }
}
