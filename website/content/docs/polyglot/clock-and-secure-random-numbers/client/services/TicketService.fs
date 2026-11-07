[<Service>]
type TicketService() =
    let clock = Clock()
    let secureRandom = SecureRandom()
    member val Ticket : string = "" with get, set
    member val IssuedAt : float = 0.0 with get, set

    member this.Issue() : unit =
        this.Ticket <- secureRandom.Uuid()
        this.IssuedAt <- clock.NowMs()
