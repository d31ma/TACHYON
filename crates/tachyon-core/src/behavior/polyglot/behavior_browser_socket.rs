pub struct BehaviorBrowserSocket;

impl BehaviorBrowserSocket {
    const CODES: [&'static str; 3] = ["HTTP_MISSING", "HTTP_REQUEST", "CANCELLED"];
    const NETWORK: &'static str = "HTTP_NETWORK";
    const NOT_SENT: &'static str = "The socket message was not sent.";

    pub async fn send<T: TacModel>(client: &str, message: T) -> BehaviorResult<()> {
        let answer = Self::host(client, message).await?;
        Self::answered(&answer)
    }

    fn answered(answer: &str) -> BehaviorResult<()> {
        let mut reader = BehaviorJsonReader {
            text: answer,
            at: 0,
        };
        let parsed = reader.value(0);
        let sent = parsed
            .as_ref()
            .and_then(|value| value.get("value"))
            .is_some();
        let error = parsed.as_ref().and_then(|value| value.get("error"));
        if [sent, error.is_none()] == [true, true] {
            return Ok(());
        }
        Err(behavior_error(Self::code(error), Self::NOT_SENT))
    }

    fn code(error: Option<&BehaviorValue>) -> &'static str {
        let named = error.and_then(|error| error.get("code"));
        Self::CODES
            .into_iter()
            .find(|code| named == Some(&BehaviorValue::from(*code)))
            .unwrap_or(Self::NETWORK)
    }

    #[cfg(target_arch = "wasm32")]
    async fn host<T: TacModel>(client: &str, message: T) -> BehaviorResult<String> {
        const MESSAGE_BYTES: usize = 65_536;
        let text = tac_write(&message.write());
        if text.len() > MESSAGE_BYTES {
            return Err(behavior_error(
                "HTTP_BODY",
                "A socket message is JSON of at most 64 KiB.",
            ));
        }
        let owner = tac_executing()
            .with(|slot| slot.borrow().as_ref().map(|active| active.owner.clone()))
            .ok_or_else(|| behavior_error("HTTP_REQUEST", Self::NOT_SENT))?;
        let request = format!("{{\"client\":{},\"message\":{text}}}", tac_quote(client));
        Ok(TacHttpTicket::socket(&owner, &request).await)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn host<T: TacModel>(client: &str, message: T) -> BehaviorResult<String> {
        TacPageSocket::send(client, message.write())
            .map(|()| String::from("{\"value\":null}"))
            .map_err(|code| behavior_error(code, Self::NOT_SENT))
    }
}
