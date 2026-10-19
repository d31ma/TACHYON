[<Store(typeof<ProgramNoteEntity>, {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})>]
type ProgramNoteStore =
    inherit Repository<ProgramNoteEntity, string>
