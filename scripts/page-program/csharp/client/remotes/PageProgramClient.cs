[HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})]
public interface PageProgramClient
{
    [GetRequest("/read")]
    System.Threading.Tasks.Task<Models.ProgramState> read();

    [PostRequest("/increment")]
    System.Threading.Tasks.Task<Models.ProgramState> increment();
}
