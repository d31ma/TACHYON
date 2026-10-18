final class TacHost {
    private TacHost() {
    }

    @org.teavm.jso.JSBody(params = "payload", script = "return globalThis[Symbol.for('tachyon.java.host')].emit(payload);")
    static native void emit(String payload);

    @org.teavm.jso.JSBody(params = "text", script = "return globalThis[Symbol.for('tachyon.java.host')].storage(text);")
    static native String storage(String text);

    @org.teavm.jso.JSBody(params = "text", script = "return globalThis[Symbol.for('tachyon.java.host')].web(text);")
    static native String web(String text);

    @org.teavm.jso.JSBody(params = "milliseconds", script = "return globalThis[Symbol.for('tachyon.java.host')].pause(milliseconds);")
    static native void pause(double milliseconds);

    @org.teavm.jso.JSBody(params = {"ticket", "text"}, script = "return globalThis[Symbol.for('tachyon.java.host')].httpStart(ticket, text);")
    static native void httpStart(int ticket, String text);

    @org.teavm.jso.JSBody(params = "ticket", script = "return globalThis[Symbol.for('tachyon.java.host')].httpCancel(ticket);")
    static native void httpCancel(int ticket);

    @org.teavm.jso.JSBody(params = {"ticket", "owner", "text"}, script = "return globalThis[Symbol.for('tachyon.java.host')].socket(ticket, owner, text);")
    static native void socket(int ticket, String owner, String text);

    @org.teavm.jso.JSBody(params = {"task", "text"}, script = "return globalThis[Symbol.for('tachyon.java.host')].taskDone(task, text);")
    static native void taskDone(int task, String text);
}
