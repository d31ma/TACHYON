final class TacStream {
    final Object description;

    TacStream(String description) {
        this.description = TacJsonReader.read(description);
    }
}
