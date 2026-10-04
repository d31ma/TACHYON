open Models

[<Service>]
type UserListService() =
    member val Names : string[] = [||] with get, set

    [<OnEvent>]
    member this.UserSaved(saved: UserSaved) : unit =
        this.Names <- Array.append this.Names [| saved.Name |]
