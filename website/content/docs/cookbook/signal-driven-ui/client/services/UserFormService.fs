open System.Threading.Tasks
open Models

[<Service>]
type UserFormService(applicationEventPublisher: ApplicationEventPublisher) =
    let usersClient = UsersClient()
    member val Name = "" with get, set

    member this.Save() : Task =
        task {
            let! answer = usersClient.Save(this.Name)
            applicationEventPublisher.PublishEvent({ Name = answer.Get("name").Text }, false)
        }
