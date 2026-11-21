open Models

[<RequestRoute("/me")>]
[<SignedIn>]
type MeController() =
    [<GetRoute>]
    member this.GetMe([<FromIdentity>] caller: Identity) : string =
        caller.User
