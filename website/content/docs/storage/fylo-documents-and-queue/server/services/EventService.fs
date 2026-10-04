[<Service>]
type EventService(dataSource: DataSource) =
    static let collection = "hd-events"
    static let deploys = {| kind = "deploy" |}
    static let change = {| count = 7 |}
    static let pageSize = 2
    static let topic = "app.render"
    static let group = "renderers"

    member this.Report() : obj =
        dataSource.CreateCollection(collection) |> ignore
        let id = dataSource.Insert(collection, deploys).GetProperty("id").GetString()
        dataSource.Insert(collection, deploys) |> ignore
        dataSource.Insert(collection, deploys) |> ignore
        let found = dataSource.FindById(collection, id)
        let patched = dataSource.Patch(collection, id, change)
        let pages, seen = this.CountPages()
        let job = this.RunJob()
        dataSource.DeleteById(collection, id) |> ignore
        let gone = dataSource.FindById(collection, id).ValueKind = System.Text.Json.JsonValueKind.Null
        let dropped = dataSource.DropCollection(collection).GetBoolean()
        box (dict [
            "found", box (found.GetProperty("kind").GetString())
            "patched", box (patched.GetProperty("count").GetInt32())
            "pages", box pages
            "seen", box seen
            "job", box job
            "gone", box gone
            "dropped", box dropped ])

    member private this.CountPages() : int * int =
        let mutable pages = 0
        let mutable seen = 0
        let mutable cursor : string = null
        let mutable more = true
        while more do
            let window = dataSource.Scroll(collection, deploys, cursor, pageSize)
            pages <- pages + 1
            seen <- seen + window.GetProperty("content").GetArrayLength()
            cursor <- window.GetProperty("nextCursor").GetString()
            more <- not (isNull cursor)
        (pages, seen)

    member private this.RunJob() =
        dataSource.Publish(topic, {| app = "web" |}, "render:web") |> ignore
        let delivery = dataSource.Claim(topic, group, 1, 30000).[0]
        dataSource.Ack(topic, group, delivery) |> ignore
        delivery.GetProperty("payload")
