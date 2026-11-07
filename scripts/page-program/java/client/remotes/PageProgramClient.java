import models.ProgramState;

@HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})
interface PageProgramClient {
    @GetRequest("/read")
    ProgramState read();

    @PostRequest("/increment")
    ProgramState increment();
}
