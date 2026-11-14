open System.Net.Http
open Models

[<IdentityProvider>]
type AppIdentityProvider() =
    static let brokenToken = "broken-token"

    static let sessions =
        dict [
            "ada-token", { User = "ada"; Roles = [| "Admin" |] }
            "bob-token", { User = "bob"; Roles = [||] }
        ]

    member this.Identify(request: HttpRequestMessage) : Identity option =
        let token = string request.Headers.Authorization
        this.RefuseBroken(token)
        this.Find(token)

    member private this.RefuseBroken(token: string) : unit =
        if token = brokenToken then failwith "the session store is down"

    member private this.Find(token: string) : Identity option =
        match sessions.TryGetValue(token) with
        | true, identity -> Some identity
        | _ -> None
