#[Setup]
struct UsersSetup {}
impl UsersSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        struct UsersAdapter {
            saved: std::sync::Mutex<Vec<BehaviorValue>>,
        }
        impl BehaviorHttpAdapter for UsersAdapter {
            fn exchange(&self, request: &BehaviorHttpRequest, _execution: &BehaviorExecution) -> BehaviorResult<BehaviorValue> {
                let mut saved = self.saved.lock().map_err(|_| BehaviorError::new("state", "the saved users are unavailable"))?;
                if request.method == "GET" {
                    return Ok(BehaviorValue::Array(saved.clone()));
                }
                let name = request.body.clone().unwrap_or(BehaviorValue::Null);
                let user = BehaviorValue::object([("name", name)]);
                saved.push(user.clone());
                Ok(user)
            }
        }

        let adapter = UsersAdapter { saved: std::sync::Mutex::new(Vec::new()) };
        let mut configuration = BehaviorConfiguration::default();
        configuration.http.insert("users".into(), std::sync::Arc::new(adapter));
        Ok(configuration)
    }
}
