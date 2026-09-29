final class TacHost {
    private TacHost() {
    }

    @org.teavm.jso.JSBody(params = "payload", script = "globalThis[Symbol.for('tachyon.java.host')].emit(payload);")
    static native void emit(String payload);

    @org.teavm.jso.JSBody(params = "text", script = "return globalThis[Symbol.for('tachyon.java.host')].storage(text);")
    static native String storage(String text);
}
