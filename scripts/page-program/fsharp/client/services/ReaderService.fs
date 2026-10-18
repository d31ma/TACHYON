open System.Threading.Tasks

[<Service>]
type ReaderService(pageProgramClient: PageProgramClient, programNoteStore: ProgramNoteStore) =
    static let noteId: string = "note"
    static let textField: string = "text"

    member val Identity : string = "" with get, set
    member val Count : int = 0 with get, set
    member val Recalled : string = "" with get, set

    member this.Inspect() : Task =
        task {
            let! state = pageProgramClient.Read()
            this.Show(state)
        }

    member this.Increment() : Task =
        task {
            let! state = pageProgramClient.Increment()
            this.Show(state)
        }

    member this.Remember(text: string) : Task =
        task {
            let note = BehaviorJSON.Object(dict [ "id", BehaviorJSON.From(noteId); textField, BehaviorJSON.From(text) ])
            let! _ = programNoteStore.save(note)
            return ()
        }

    member this.Recall() : Task =
        task {
            let! note = programNoteStore.findById(BehaviorJSON.From(noteId))
            this.Recalled <- this.TextOf(note)
        }

    member private this.TextOf(note: BehaviorJSON) : string =
        if note.Has(textField) then note.Get(textField).Text else ""

    member private this.Show(state: Models.ProgramState) : unit =
        this.Identity <- state.Identity
        this.Count <- state.Count
