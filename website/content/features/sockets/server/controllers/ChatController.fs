open Models

[<SocketRoute("/chat")>]
[<Public>]
type ChatController() =
    [<OnOpen>]
    member this.OnOpen(session: SocketSession) : unit =
        session.Send({ Text = "Welcome to the room" })

    [<OnMessage>]
    member this.OnMessage(session: SocketSession, message: ChatMessage) : unit =
        ChatController.SendToEveryone(session.OpenSessions(), message)

    static member private SendToEveryone(peers: ResizeArray<SocketPeer>, message: ChatMessage) : unit =
        for peer in peers do
            peer.Send(message)
