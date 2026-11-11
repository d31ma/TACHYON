[<Service>]
type AttachService() =
    let files = Files()
    let mutable selected = ""
    member val Name : string = "" with get, set
    member val Digest : string = "" with get, set
    member val Status : string = "" with get, set
    member val Problem : string = "" with get, set

    member this.Choose() : unit = files.Pick(".pdf,image/*", false)

    member this.Upload() : unit =
        files.Upload(selected, "PUT", "https://uploads.example.com/inbox", "content-type: application/octet-stream") |> ignore

    [<OnEvent>]
    member this.Picked(picked: FilePicked) : unit =
        this.Name <- $"{picked.Name}, {picked.Size} bytes"
        this.Digest <- picked.Sha256
        selected <- picked.Id

    [<OnEvent>]
    member this.Finished(finished: UploadFinished) : unit =
        this.Status <- $"HTTP {finished.Status} {finished.Error}"
