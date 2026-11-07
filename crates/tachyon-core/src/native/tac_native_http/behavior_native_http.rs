#[cfg(not(target_arch = "wasm32"))]
struct BehaviorNativeHttp;

#[cfg(not(target_arch = "wasm32"))]
impl BehaviorNativeHttp {
    const MOST_DEPTH: usize = 32;
    const MOST_ANSWER: usize = 1_048_576;
    const SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    const EMPTY: &'static str = "{\"value\":null}";
    const RESPONSE: &'static str = "HTTP_RESPONSE";
    const NETWORK: &'static str = "HTTP_NETWORK";
    const LIMIT: &'static str = "HTTP_LIMIT";
    const SUCCESS: core::ops::Range<u16> = 200..300;
    const REDIRECT: core::ops::Range<u16> = 300..400;

    async fn send(text: String) -> BehaviorResult<String> {
        let started = BehaviorNativeHttpRequest::read(&text).map(TacNativeHttp::start);
        let answer = match started {
            Ok(exchange) => Self::answer(exchange.await),
            Err(code) => Self::failure(code),
        };
        Ok(answer)
    }

    fn answer(outcome: Result<TacNativeHttpResponse, &'static str>) -> String {
        outcome
            .map_err(Self::failure)
            .and_then(Self::accepted)
            .and_then(Self::value)
            .unwrap_or_else(|failure| failure)
    }

    fn accepted(response: TacNativeHttpResponse) -> Result<TacNativeHttpResponse, String> {
        let status = response.status;
        match [Self::SUCCESS.contains(&status), Self::REDIRECT.contains(&status)] {
            [true, _] => Ok(response),
            [_, true] => Err(Self::failure(Self::NETWORK)),
            _ => Err(format!(
                "{{\"error\":{{\"code\":\"HTTP_STATUS\",\"status\":{status}}}}}"
            )),
        }
    }

    fn value(response: TacNativeHttpResponse) -> Result<String, String> {
        let text = String::from_utf8(response.body).map_err(|_| Self::failure(Self::RESPONSE))?;
        let trimmed = text.trim();
        match [trimmed.is_empty(), Self::json(trimmed)] {
            [true, _] => Ok(String::from(Self::EMPTY)),
            [false, true] => Self::bounded(format!("{{\"value\":{trimmed}}}")),
            [false, false] => Err(Self::failure(Self::RESPONSE)),
        }
    }

    fn bounded(answer: String) -> Result<String, String> {
        (answer.len() <= Self::MOST_ANSWER)
            .then_some(answer)
            .ok_or_else(|| Self::failure(Self::LIMIT))
    }

    fn json(text: &str) -> bool {
        let mut reader = BehaviorJsonReader { text, at: 0 };
        let value = reader.value(0);
        reader.skip_space();
        let whole = reader.at == text.len();
        value.is_some_and(|value| [whole, Self::plain(&value, 0)] == [true, true])
    }

    fn plain(value: &BehaviorValue, depth: usize) -> bool {
        let nested = depth <= Self::MOST_DEPTH;
        let inner = match value {
            BehaviorValue::Number(number) => Self::safe_number(*number),
            BehaviorValue::Array(items) => items.iter().all(|item| Self::plain(item, depth + 1)),
            BehaviorValue::Object(entries) => {
                entries.values().all(|item| Self::plain(item, depth + 1))
            }
            _ => true,
        };
        [nested, inner] == [true, true]
    }

    fn safe_number(number: f64) -> bool {
        [number.is_finite(), number.abs() <= Self::SAFE_INTEGER] == [true, true]
    }

    fn failure(code: &str) -> String {
        format!("{{\"error\":{{\"code\":\"{code}\"}}}}")
    }
}
