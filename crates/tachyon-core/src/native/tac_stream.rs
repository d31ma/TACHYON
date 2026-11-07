pub struct TacStream {
    pub client: &'static str,
    pub path: &'static str,
    pub event: &'static str,
    pub socket: bool,
    pub models: Option<&'static str>,
}

impl TacStream {
    fn describe(&self) -> TacValue {
        let models = self
            .models
            .map_or(TacValue::Null, |models| TacScanner::new(models).value());
        TacValue::Object(vec![
            (
                String::from("client"),
                TacValue::Text(self.client.to_owned()),
            ),
            (String::from("path"), TacValue::Text(self.path.to_owned())),
            (String::from("event"), TacValue::Text(self.event.to_owned())),
            (String::from("socket"), TacValue::Flag(self.socket)),
            (String::from("models"), models),
        ])
    }
}
