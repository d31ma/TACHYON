open System.Net.Http
open Models

[<IdentityProvider>]
type AppIdentityProvider() =
    static let sessions =
        dict [
            "ada-token", { User = "ada"; Roles = [| "Admin" |] }
            "bob-token", { User = "bob"; Roles = [||] }
        ]

    member this.Identify(request: HttpRequestMessage) : Identity option =
        this.Find(string request.Headers.Authorization)

    member private this.Find(token: string) : Identity option =
        match sessions.TryGetValue(token) with
        | true, identity -> Some identity
        | _ -> None
