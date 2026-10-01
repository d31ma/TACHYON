#[derive(Debug)]
struct NativeCapabilityScope {
    owner: String,
    active: core::cell::Cell<bool>,
    adapter: NativeCapabilityAdapter,
}

impl NativeCapabilityScope {
    fn new(owner: &str, adapter: NativeCapabilityAdapter) -> Self {
        Self {
            owner: owner.to_owned(),
            active: core::cell::Cell::new(!owner.is_empty()),
            adapter,
        }
    }

    fn call(&self, operation: &str, values: Vec<(String, TacValue)>) -> String {
        if !self.active.get() {
            return String::from("{\"status\":\"disposed\"}");
        }
        let mut values = values;
        values.push((String::from("owner"), TacValue::Text(self.owner.clone())));
        self.adapter
            .invoke(operation, &tac_write(&TacValue::Object(values)))
    }

    fn dispose(&self) {
        if !self.active.replace(false) {
            return;
        }
        let request = TacValue::Object(vec![(
            String::from("owner"),
            TacValue::Text(self.owner.clone()),
        )]);
        let _ = self
            .adapter
            .invoke("filePicker.dispose", &tac_write(&request));
    }
}
