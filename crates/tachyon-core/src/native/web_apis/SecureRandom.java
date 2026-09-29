final class SecureRandom {
    private static final String API = "secureRandom";

    byte[] bytes(int count) {
        java.util.List<?> values = TacWebHost.list(TacWebHost.call(API, "bytes", count));
        byte[] bytes = new byte[values.size()];
        for (int index = 0; index < bytes.length; index++) {
            bytes[index] = (byte) TacWebHost.whole(values.get(index));
        }
        return bytes;
    }

    String uuid() {
        return TacWebHost.text(API, "uuid");
    }
}
