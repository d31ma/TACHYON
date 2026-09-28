public sealed class AnnotatedValidationService {
    public int calls = 0;
    public BehaviorJSON accept([Check(typeof(AnnotatedInput))] BehaviorJSON input) {
        calls++;
        return input;
    }
}

