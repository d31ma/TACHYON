import java.util.Map;
import models.ProgramState;

@Service
class WriterService {
    private static final String ID = "id";
    private static final String NOTE = "note";
    private static final String TEXT = "text";
    @Needs
    private final PageProgramClient pageProgramClient;
    @Needs
    private final ProgramNoteStore programNoteStore;
    String identity = "";
    int count = 0;
    String recalled = "";

    void inspect() {
        show(pageProgramClient.read());
    }

    void increment() {
        show(pageProgramClient.increment());
    }

    void remember(String text) {
        programNoteStore.save(Map.of(ID, NOTE, TEXT, text));
    }

    void recall() {
        recalled = textOf(programNoteStore.findById(NOTE));
    }

    private void show(ProgramState state) {
        identity = state.identity();
        count = state.count();
    }

    private String textOf(Object note) {
        return note instanceof Map<?, ?> found ? String.valueOf(found.get(TEXT)) : "";
    }
}
