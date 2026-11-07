import models.ProgramState

@HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})
interface PageProgramClient {
    @GetRequest("/read")
    suspend fun read(): ProgramState

    @PostRequest("/increment")
    suspend fun increment(): ProgramState
}
