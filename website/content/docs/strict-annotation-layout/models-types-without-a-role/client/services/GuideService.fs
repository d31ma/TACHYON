open Models

type GuideService() =
    let guide = { Title = "Models hold types"; Pages = 3 }

    member this.Title : string = guide.Title

    member this.Pages : int = guide.Pages
