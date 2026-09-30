@Service
class RuntimeNoteService {
    final String runtime = "Java, compiled to WebAssembly by TeaVM";

    boolean showsNotes(boolean development) {
        return development;
    }
}
