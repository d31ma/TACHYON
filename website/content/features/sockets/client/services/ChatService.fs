open System.Threading.Tasks
open Models

[<Service>]
type ChatService() =
    let messageLimit : int = 50

    [<Needs>]
    member val ChatClient : ChatClient = Unchecked.defaultof<ChatClient> with get, set

    member val Messages : ChatMessage list = [] with get, set

    [<OnEvent>]
    member this.AddMessage(message: ChatMessage) : unit =
        let messages = this.Messages @ [ message ]
        this.Messages <- List.skip (max 0 (messages.Length - messageLimit)) messages

    member this.SayHello() : Task =
        this.ChatClient.Send({ Text = "Hello from a tab" })
