use crate::models::ProgramState;

#[Service]
pub struct ReaderService {
    #[Needs]
    page_program_client: std::sync::Arc<PageProgramClient>,
    #[Needs]
    program_note_store: std::sync::Arc<ProgramNoteStore>,
    pub identity: String,
    pub count: i64,
    pub recalled: String,
}

impl ReaderService {
    const NOTE: &'static str = "note";
    const TEXT: &'static str = "text";

    pub async fn inspect(&mut self) -> BehaviorResult<()> {
        let state = self.page_program_client.read().await?;
        self.show(state);
        Ok(())
    }

    pub async fn increment(&mut self) -> BehaviorResult<()> {
        let state = self.page_program_client.increment().await?;
        self.show(state);
        Ok(())
    }

    pub async fn remember(&mut self, text: String) -> BehaviorResult<()> {
        let note = BehaviorValue::object([("id", BehaviorValue::from(Self::NOTE)), (Self::TEXT, BehaviorValue::String(text))]);
        self.program_note_store.save(note)?;
        Ok(())
    }

    pub async fn recall(&mut self) -> BehaviorResult<()> {
        let note = self.program_note_store.find_by_id(BehaviorValue::from(Self::NOTE))?;
        self.recalled = Self::text_of(&note);
        Ok(())
    }

    fn text_of(note: &BehaviorValue) -> String {
        match note.get(Self::TEXT) {
            Some(BehaviorValue::String(text)) => text.clone(),
            _ => String::new(),
        }
    }

    fn show(&mut self, state: ProgramState) {
        self.identity = state.identity;
        self.count = state.count;
    }
}
