@Service
class EditorService {
    @SessionState
    var draft: String = ""
    @LocalState
    var theme: String = "light"
}
