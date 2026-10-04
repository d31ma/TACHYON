open System.Threading.Tasks

[<Helper>]
type CatalogAdapterHelper() =
    let rows = BehaviorJSON.Parse("[{\"id\":1,\"status\":\"active\"},{\"id\":2,\"status\":\"retired\"}]")

    interface BehaviorEntityAdapter with
        member this.execute(command: BehaviorJSON, execution: BehaviorExecution) : Task<BehaviorJSON> =
            let query = command.Get("query")
            let condition = query.Get("where").Items[0]
            let keepUnequalRows = condition.Get("operator").Text = "ne"
            let isMatch (row: BehaviorJSON) =
                let isEqual = row.Get(condition.Get("field").Text).ToString() = condition.Get("value").ToString()
                if keepUnequalRows then not isEqual else isEqual
            let matchingRows = rows.Items |> Array.filter isMatch
            if query.Get("kind").Text = "count" then
                Task.FromResult(BehaviorJSON.From(matchingRows.Length))
            else
                Task.FromResult(BehaviorJSON.Array(matchingRows))
