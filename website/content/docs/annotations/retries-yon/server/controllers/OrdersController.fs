open System.Text.Json.Nodes
open System.Threading.Tasks
open Models

[<RequestRoute("/orders")>]
[<Public>]
type OrdersController(orders: OrdersService) =
    [<GetRoute>]
    member this.GetOrders() : OrderReport = this.BuildReport().Result

    member private this.BuildReport() : Task<OrderReport> =
        task {
            let id = BehaviorJSON.From("A-1")
            let! submitted = orders.submit(id)
            let! report = orders.report(id)
            return
                { Submitted = JsonNode.Parse(submitted.ToString())
                  Report = JsonNode.Parse(report.ToString()) }
        }
