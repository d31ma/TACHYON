final class TacField {
    final java.util.function.Supplier<Object> read;
    final java.util.function.Consumer<Object> write;

    TacField(java.util.function.Supplier<Object> read, java.util.function.Consumer<Object> write) {
        this.read = read;
        this.write = write;
    }
}
