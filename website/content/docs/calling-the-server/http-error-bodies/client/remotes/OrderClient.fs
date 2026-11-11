open System.Threading.Tasks
open Models

[<HTTPClient("/api")>]
type OrderClient =
    [<PostRequest("/orders")>]
    abstract send: [<RequestBody>] order: BehaviorJSON -> Task<OrderInput>
