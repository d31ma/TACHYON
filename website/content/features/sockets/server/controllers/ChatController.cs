using Models;

[SocketRoute("/chat")]
[Public]
public class ChatController
{
    [OnOpen]
    public void OnOpen(SocketSession session)
    {
        session.Send(new ChatMessage("Welcome to the room"));
    }

    [OnMessage]
    public void OnMessage(SocketSession session, ChatMessage message)
    {
        SendToEveryone(session.OpenSessions(), message);
    }

    private static void SendToEveryone(List<SocketPeer> peers, ChatMessage message)
    {
        foreach (var peer in peers) peer.Send(message);
    }
}
