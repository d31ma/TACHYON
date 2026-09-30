@Service
class EditorService {
    @SessionState
    String draft = "";
    @LocalState
    String theme = "light";
}
