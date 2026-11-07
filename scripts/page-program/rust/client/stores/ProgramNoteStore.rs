#[Store(ProgramNoteEntity, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})]
trait ProgramNoteStore: Repository<ProgramNoteEntity, String> {}
