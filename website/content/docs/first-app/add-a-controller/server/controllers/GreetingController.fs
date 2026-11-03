open Models

[<RequestRoute("/api/greeting")>]
[<Public>]
type GreetingController() =
    [<GetRoute>]
    member this.GetGreeting() : Greeting =
        { Message = "Hello from Yon" }
