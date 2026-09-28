open Models

[<RequestRoute("/echo")>]
[<Public>]
type EchoController() =
    [<PostRoute>]
    member this.PostEcho([<FromBody>] message: Message) : Message =
        message
