open System.Threading.Tasks
open Models

[<Service>]
type SettingsService() =
    let applicationClient = ApplicationClient()
    member val api : string = "" with get, set

    member this.Load() : Task =
        task {
            let! application = applicationClient.Load()
            this.api <- Behaviors.values(typeof<PublicSettings>, application).Get("api").Text
        }
