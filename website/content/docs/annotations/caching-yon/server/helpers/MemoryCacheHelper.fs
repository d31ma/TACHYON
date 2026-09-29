open System.Collections.Generic
open System.Threading.Tasks

[<Helper>]
type MemoryCacheHelper() =
    let values = Dictionary<string, BehaviorJSON>()

    interface BehaviorCacheAdapter with
        member this.get(key: string, execution: BehaviorExecution) : Task<BehaviorJSON> =
            match values.TryGetValue(key) with
            | true, value -> Task.FromResult(value)
            | _ -> Task.FromResult(BehaviorJSON.Parse("{\"hit\":false}"))

        member this.set(key: string, value: BehaviorJSON, policy: BehaviorJSON, expiresAt: int64, execution: BehaviorExecution) : Task =
            values[key] <- BehaviorJSON.Object(dict [ "hit", BehaviorJSON.From(true); "value", value; "expiresAt", BehaviorJSON.From(expiresAt) ])
            Task.CompletedTask

        member this.delete(key: string, execution: BehaviorExecution) : Task =
            values.Remove(key) |> ignore
            Task.CompletedTask
