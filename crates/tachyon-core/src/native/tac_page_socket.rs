pub struct TacPageSocket;

impl TacPageSocket {
    const MESSAGE_BYTES: usize = 65_536;
    const MISSING: &'static str = "HTTP_MISSING";
    const NO_OWNER: &'static str = "HTTP_REQUEST";
    const TOO_LARGE: &'static str = "HTTP_BODY";

    pub fn send(client: &str, message: TacValue) -> Result<(), &'static str> {
        let text = tac_write(&message);
        let owner = tac_executing()
            .with(|slot| slot.borrow().clone())
            .ok_or(Self::NO_OWNER)?;
        let emit = tac_emit_slot()
            .lock()
            .ok()
            .and_then(|installed| *installed)
            .ok_or(Self::MISSING)?;
        let fits = text.len() <= Self::MESSAGE_BYTES;
        fits.then_some(()).ok_or(Self::TOO_LARGE)?;
        let payload = Self::payload(&owner, client, &text);
        let owned = std::ffi::CString::new(payload).map_err(|_| Self::TOO_LARGE)?;
        emit(owned.as_ptr());
        Ok(())
    }

    fn payload(owner: &TacActivation, client: &str, text: &str) -> String {
        format!(
            "{{\"route\":{},\"service\":{},\"owner\":{},\"send\":{},\"value\":{text}}}",
            tac_quote(&owner.route),
            tac_quote(&owner.service),
            tac_quote(&owner.owner),
            tac_quote(client)
        )
    }
}
