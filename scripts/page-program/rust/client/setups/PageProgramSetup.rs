#[Setup]
struct PageProgramSetup {}
impl PageProgramSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct ProgramAdapter {
            identity: String,
            count: std::sync::Mutex<f64>,
        }
        impl BehaviorHttpAdapter for ProgramAdapter {
            fn exchange(&self, request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                let mut count = self.count.lock().map_err(|_| BehaviorError::new("state", "the program count is unavailable"))?;
                if request.method == "POST" && request.path.ends_with("/increment") {
                    *count += 1.0;
                }
                Ok(BehaviorValue::object([
                    ("identity", BehaviorValue::String(self.identity.clone())),
                    ("count", BehaviorValue::Number(*count)),
                ]))
            }
        }

        let identity = SecureRandom::default().uuid().map_err(|_| BehaviorError::new("random", "a random identity is unavailable"))?;
        let adapter = ProgramAdapter { identity, count: std::sync::Mutex::new(0.0) };
        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("pageprogram".into(), std::sync::Arc::new(adapter));
        Ok(configuration)
    }
}
