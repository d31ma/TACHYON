@Service
final class EditorService {
    @SessionState
    var draft: String = ""
    @LocalState
    var theme: String = "light"
}
