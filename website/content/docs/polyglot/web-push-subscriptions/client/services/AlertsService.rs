#[Service]
#[derive(Default)]
struct AlertsService {
    push: Push,
    endpoint: String,
    problem: String,
}

impl AlertsService {
    const SERVER_KEY: &'static str = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U";

    pub fn turn_on(&mut self) {
        let answer = self.push.subscribe(Self::SERVER_KEY);
        self.report(answer);
    }

    pub fn turn_off(&mut self) {
        let answer = self.push.unsubscribe();
        self.report(answer);
    }

    fn report(&mut self, answer: Result<(), String>) {
        if let Err(message) = answer {
            self.problem = message;
        }
    }

    #[OnEvent]
    fn subscribed(&mut self, subscribed: PushSubscribed) {
        self.endpoint = subscribed.endpoint;
    }

    #[OnEvent]
    fn unsubscribed(&mut self, _unsubscribed: PushUnsubscribed) {
        self.endpoint = String::new();
    }

    #[OnEvent]
    fn failed(&mut self, failed: BrowserCallFailed) {
        self.problem = failed.message;
    }
}
