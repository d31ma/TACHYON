@Service
final class RuntimeNoteService {
    let runtime: String = "Swift, in the app or as WebAssembly"

    func showsNotes(_ development: Bool) -> Bool {
        development
    }
}
