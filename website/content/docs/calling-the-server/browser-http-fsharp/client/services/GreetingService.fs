open System.Threading.Tasks
open Models

[<Service>]
type GreetingService() =
    [<Needs>]
    member val GreetingClient : GreetingClient = Unchecked.defaultof<GreetingClient> with get, set

    [<Needs>]
    member val OverrideClient : OverrideClient = Unchecked.defaultof<OverrideClient> with get, set

    member val Message : string = "" with get, set
    member val Custom : string = "" with get, set
    member val Loads : int = 0 with get, set
    member val Clears : int = 0 with get, set

    member this.Ask() : Task<Greeting> =
        task {
            this.Loads <- this.Loads + 1
            let! greeting = this.GreetingClient.read()
            this.Message <- greeting.Message
            return greeting
        }

    member this.Clear() : Task<Greeting> =
        task {
            let! answer = this.GreetingClient.clear()
            this.Clears <- this.Clears + 1
            return answer
        }

    member this.AskCustom() : Task<Greeting> =
        task {
            let! greeting = this.OverrideClient.read()
            this.Custom <- greeting.Message
            return greeting
        }
