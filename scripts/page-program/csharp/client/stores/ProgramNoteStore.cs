[Store(typeof(ProgramNoteEntity), {"fields":["id","text"],"defaultSize":20,"maxSize":100,"setup":"PageProgramSetup"})]
public interface ProgramNoteStore : Repository<ProgramNoteEntity, string>
{
}
