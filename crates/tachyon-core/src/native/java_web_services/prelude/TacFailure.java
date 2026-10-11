abstract class TacFailure extends RuntimeException {
    final String code;

    TacFailure(String code, String message) {
        super(message);
        this.code = code;
    }
}
