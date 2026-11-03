pub struct BehaviorBrowserHttp;

impl BehaviorBrowserHttp {
    const CODES: [&'static str; 13] = [
        "HTTP_REQUEST",
        "HTTP_ORIGIN",
        "HTTP_HEADER",
        "HTTP_QUERY",
        "HTTP_BODY",
        "HTTP_LIMIT",
        "HTTP_RESPONSE",
        "HTTP_STATUS",
        "HTTP_CAPACITY",
        "HTTP_NETWORK",
        "HTTP_MISSING",
        "CANCELLED",
        "DEADLINE",
    ];
    const FAILED: &'static str = "The browser HTTP request failed.";
    const NETWORK: &'static str = "HTTP_NETWORK";
    const RESPONSE: &'static str = "HTTP_RESPONSE";
    const REQUEST_BYTES: usize = 1_048_576;

    async fn send(
        request: &BehaviorHttpRequest,
        execution: &BehaviorExecution,
        (public_read, revalidate): (bool, bool),
    ) -> BehaviorResult<BehaviorValue> {
        execution.check()?;
        let text = BehaviorValue::object(Self::fields(request, execution, public_read, revalidate))
            .to_json(Self::REQUEST_BYTES)?;
        let answer = Self::host(text).await?;
        execution.check()?;
        Self::value(&answer)
    }

    fn fields(
        request: &BehaviorHttpRequest,
        execution: &BehaviorExecution,
        public_read: bool,
        revalidate: bool,
    ) -> Vec<(&'static str, BehaviorValue)> {
        let mut fields = vec![
            ("method", BehaviorValue::from(request.method)),
            ("path", BehaviorValue::String(request.path.clone())),
            ("query", request.query.clone()),
            ("headers", request.headers.clone()),
            ("redirect", BehaviorValue::from(request.redirect)),
            ("credentials", BehaviorValue::from(request.credentials)),
            ("deadline", BehaviorValue::Number(execution.deadline as f64)),
            (
                "maxResponseBytes",
                BehaviorValue::Number(request.max_response_bytes as f64),
            ),
            ("publicRead", BehaviorValue::Bool(public_read)),
            ("revalidate", BehaviorValue::Bool(revalidate)),
        ];
        if let Some(body) = &request.body {
            fields.push(("body", body.clone()));
        }
        fields
    }

    #[cfg(target_arch = "wasm32")]
    async fn host(text: String) -> BehaviorResult<String> {
        Ok(TacHttpTicket::start(&text).await)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn host(text: String) -> BehaviorResult<String> {
        BehaviorNativeHttp::send(text).await
    }

    fn value(answer: &str) -> BehaviorResult<BehaviorValue> {
        let mut reader = BehaviorJsonReader { text: answer, at: 0 };
        let object = Self::object(reader.value(0))?;
        Self::settled(object)
    }

    fn object(
        parsed: Option<BehaviorValue>,
    ) -> BehaviorResult<std::collections::BTreeMap<String, BehaviorValue>> {
        match parsed {
            Some(BehaviorValue::Object(object)) => Ok(object),
            _ => Err(behavior_error(Self::RESPONSE, Self::FAILED)),
        }
    }

    fn settled(
        mut object: std::collections::BTreeMap<String, BehaviorValue>,
    ) -> BehaviorResult<BehaviorValue> {
        if let Some(error) = object.remove("error") {
            return Err(behavior_error(Self::code(&error), Self::FAILED));
        }
        object
            .remove("value")
            .ok_or_else(|| behavior_error(Self::RESPONSE, Self::FAILED))
    }

    fn code(error: &BehaviorValue) -> &'static str {
        let named = error.get("code");
        Self::CODES
            .into_iter()
            .find(|code| named == Some(&BehaviorValue::from(*code)))
            .unwrap_or(Self::NETWORK)
    }
}
