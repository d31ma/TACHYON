open System.Threading.Tasks

[<HTTPClient({"adapter":"users","setup":"UsersSetup"})>]
type UsersClient =
    [<PostRequest("/users")>]
    abstract Save: [<RequestBody>] name: string -> Task<BehaviorJSON>

    [<GetRequest("/users")>]
    abstract All: unit -> Task<BehaviorJSON>
