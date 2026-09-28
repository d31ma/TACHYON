struct Handler;

impl Handler {
    pub fn POST(request: &YonJson) -> YonJson {
        let body = match request.get("body") {
            Some(value) => value.clone(),
            None => YonJson::Null,
        };
        YonJson::object(vec![
            ("language", "rust".into()),
            ("action", "echo".into()),
            ("body", body),
        ])
    }

    pub fn DELETE(request: &YonJson) -> YonJson {
        let size = match request.get("body") {
            Some(value) => value.stringify().len(),
            None => 0,
        };
        YonJson::object(vec![
            ("language", "rust".into()),
            ("action", "size-confirm".into()),
            ("receivedBytes", size.into()),
        ])
    }
}
