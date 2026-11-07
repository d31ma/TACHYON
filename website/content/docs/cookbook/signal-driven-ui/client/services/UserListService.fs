open System.Threading.Tasks
open Models

[<Service>]
type UserListService() =
    let usersClient = UsersClient()
    member val Names : string[] = [||] with get, set

    [<OnEvent>]
    member this.UserSaved(saved: UserSaved) : Task<unit> =
        task {
            let! users = usersClient.All()
            this.Names <- users.Items |> Array.map (fun user -> user.Get("name").Text)
        }
