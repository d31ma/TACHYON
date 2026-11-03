open Models

[<RequestRoute("/guide")>]
[<Public>]
type GuideController() =
    [<GetRoute>]
    member this.GetGuide() : Guide =
        { Title = "Models hold types"; Pages = 3 }
