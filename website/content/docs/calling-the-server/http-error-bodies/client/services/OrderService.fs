open System.Threading.Tasks
open Models

[<Service>]
type OrderService() =
    [<Needs>]
    member val OrderClient : OrderClient = Unchecked.defaultof<OrderClient> with get, set

    member val Status : int = 0 with get, set
    member val Title : string = "" with get, set
    member val Mistakes : int = 0 with get, set

    member this.Send() : Task<unit> =
        task {
            let! failure = this.Failure(BehaviorJSON.Parse("{\"name\":\"\",\"level\":12}"))
            let problem = TacMappingModels.errorBodyProblem failure
            this.Status <- failure.Status
            this.Title <- problem |> Option.map (fun found -> found.Title) |> Option.defaultValue ""
            this.Mistakes <- problem |> Option.map (fun found -> found.Errors.Length) |> Option.defaultValue 0
        }

    member private this.Failure(order) : Task<BehaviorError> =
        task {
            try
                let! _ = this.OrderClient.send(order)
                return BehaviorError("none")
            with :? BehaviorError as error ->
                return error
        }
