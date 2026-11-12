use super::{HandlerCallLabel, HandlerLogLevel, HandlerLogRecord};
use crate::sorted_json::SortedJson;
use serde_json::Value;

pub(in crate::handler) struct HandlerLogRecords {
    events: Vec<Value>,
    dropped: usize,
    output_cut: bool,
}

impl HandlerLogRecords {
    const MAX_RECORDS: usize = 100;
    const DROPPED_EVENT: &'static str = "handler.log_dropped";

    pub(in crate::handler) fn read(
        text: &str,
        output_cut: bool,
        threshold: HandlerLogLevel,
    ) -> Self {
        let records = Self::records(text, threshold);
        let dropped = records.len().saturating_sub(Self::MAX_RECORDS);
        let events = records
            .iter()
            .take(Self::MAX_RECORDS)
            .map(HandlerLogRecord::event)
            .collect();
        Self {
            events,
            dropped,
            output_cut,
        }
    }

    pub(in crate::handler) fn events(&self, label: &HandlerCallLabel) -> Vec<Value> {
        let mut events = self.stamped(label);
        events.extend(self.dropped_event(label));
        events
    }

    pub(in crate::handler) fn write(&self, label: &HandlerCallLabel) {
        Self::print(&self.events(label));
    }

    fn records(text: &str, threshold: HandlerLogLevel) -> Vec<HandlerLogRecord> {
        text.lines()
            .filter_map(HandlerLogRecord::parse)
            .filter(|record| record.reaches(threshold))
            .collect()
    }

    fn stamped(&self, label: &HandlerCallLabel) -> Vec<Value> {
        self.events
            .iter()
            .map(|event| Self::stamped_event(event, label))
            .collect()
    }

    fn stamped_event(event: &Value, label: &HandlerCallLabel) -> Value {
        let mut stamped = event.clone();
        label.stamp(&mut stamped);
        stamped
    }

    fn dropped_event(&self, label: &HandlerCallLabel) -> Option<Value> {
        self.lost_lines().then(|| {
            let mut event = serde_json::json!({
                "event": Self::DROPPED_EVENT,
                "dropped": self.dropped,
                "output_cut": self.output_cut,
            });
            label.stamp(&mut event);
            event
        })
    }

    fn lost_lines(&self) -> bool {
        let logged = self.dropped + self.events.len();
        [self.dropped > 0, self.output_cut && logged > 0].contains(&true)
    }

    fn print(events: &[Value]) {
        for event in events {
            eprintln!("{}", SortedJson::text(event));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HandlerLogRecords;
    use crate::handler::process::{HandlerCallLabel, HandlerLogLevel};

    fn label() -> HandlerCallLabel {
        HandlerCallLabel {
            request_id: String::from("0ABC"),
            route: String::from("/orders"),
            handler: String::from("server/routes/orders/OrderController.java"),
        }
    }

    fn line(level: &str, message: &str) -> String {
        format!("{{\"event\":\"yon.log\",\"level\":\"{level}\",\"message\":\"{message}\"}}\n")
    }

    #[test]
    fn each_log_line_names_the_request_the_route_and_the_handler() {
        let text = format!("plain\n{}{}", line("info", "one"), line("debug", "hidden"));
        let events = HandlerLogRecords::read(&text, false, HandlerLogLevel::Info).events(&label());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["message"], "one");
        assert_eq!(events[0]["request_id"], "0ABC");
        assert_eq!(events[0]["route"], "/orders");
        assert_eq!(
            events[0]["handler"],
            "server/routes/orders/OrderController.java"
        );
    }

    #[test]
    fn lines_past_the_limit_are_counted_in_one_dropped_event() {
        let text = (0..103)
            .map(|index| line("info", &format!("n{index}")))
            .collect::<String>();
        let events = HandlerLogRecords::read(&text, false, HandlerLogLevel::Info).events(&label());
        assert_eq!(events.len(), 101);
        assert_eq!(events[99]["message"], "n99");
        assert_eq!(events[100]["event"], "handler.log_dropped");
        assert_eq!(events[100]["dropped"], 3);
        assert_eq!(events[100]["output_cut"], false);
        assert_eq!(events[100]["route"], "/orders");
    }

    #[test]
    fn cut_output_is_reported_only_for_a_handler_that_logs() {
        let logged = HandlerLogRecords::read(&line("warn", "kept"), true, HandlerLogLevel::Info)
            .events(&label());
        assert_eq!(logged.len(), 2);
        assert_eq!(logged[1]["event"], "handler.log_dropped");
        assert_eq!(logged[1]["dropped"], 0);
        assert_eq!(logged[1]["output_cut"], true);
        let plain = HandlerLogRecords::read("noise\n", true, HandlerLogLevel::Info);
        assert!(plain.events(&label()).is_empty());
    }

    #[test]
    fn off_writes_nothing() {
        let text = line("error", "quiet");
        let records = HandlerLogRecords::read(&text, true, HandlerLogLevel::Off);
        assert!(records.events(&label()).is_empty());
    }
}
