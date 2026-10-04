open System.Threading.Tasks

[<HTTPClient>]
type UsersClient =
    [<PostRequest("/users")>]
    abstract Save: [<RequestBody>] name: string -> Task<BehaviorJSON>
