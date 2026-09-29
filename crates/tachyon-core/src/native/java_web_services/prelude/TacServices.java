public class TacServices {
    public static void main(String[] arguments) {
    }

    @org.teavm.jso.JSExport
    public static String tacInvoke(String request) {
        return TacBridge.handle(request);
    }
}
