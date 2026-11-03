open Models

[<HTTPClient("/chat")>]
type ChatClient =
    [<SocketRequest>]
    abstract Receive: ChatMessage -> unit
