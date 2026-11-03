open System.Threading.Tasks

[<HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})>]
type PageProgramClient =
    [<GetRequest("/read")>]
    abstract Read: unit -> Task<Models.ProgramState>

    [<PostRequest("/increment")>]
    abstract Increment: unit -> Task<Models.ProgramState>
