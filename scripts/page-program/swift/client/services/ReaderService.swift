@Service
final class ReaderService {
    private static let noteId = "note"
    private static let textField = "text"
    @Needs
    private let pageProgramClient: PageProgramClient
    @Needs
    private let programNoteStore: ProgramNoteStore
    var identity: String = ""
    var count: Int = 0
    var recalled: String = ""

    func inspect() async throws {
        show(try await pageProgramClient.read())
    }

    func increment() async throws {
        show(try await pageProgramClient.increment())
    }

    func remember(_ text: String) async throws {
        _ = try await programNoteStore.save(.object(["id": .string(Self.noteId), Self.textField: .string(text)]))
    }

    func recall() async throws {
        recalled = Self.textOf(try await programNoteStore.findById(.string(Self.noteId)))
    }

    private func show(_ state: ProgramState) {
        identity = state.identity
        count = state.count
    }

    private static func textOf(_ note: BehaviorJSON) -> String {
        (try? note.get(textField).text) ?? ""
    }
}
