final class TacJsonWriter {
    private TacJsonWriter() {
    }

    static String write(Object value) {
        StringBuilder written = new StringBuilder();
        append(written, value);
        return written.toString();
    }

    static String quote(String text) {
        StringBuilder written = new StringBuilder();
        appendText(written, text);
        return written.toString();
    }

    private static void append(StringBuilder written, Object value) {
        if (value == null) {
            written.append("null");
        } else if (value instanceof Boolean || value instanceof Integer || value instanceof Long
                || value instanceof Short || value instanceof Byte) {
            written.append(value);
        } else if (value instanceof Number number) {
            appendDecimal(written, number.doubleValue());
        } else if (value instanceof java.util.Map<?, ?> map) {
            appendMap(written, map);
        } else if (value instanceof Iterable<?> items) {
            appendItems(written, items);
        } else if (value instanceof Object[] items) {
            appendItems(written, java.util.Arrays.asList(items));
        } else {
            appendText(written, value.toString());
        }
    }

    private static void appendDecimal(StringBuilder written, double number) {
        if (Double.isNaN(number) || Double.isInfinite(number)) {
            written.append("null");
        } else if (number == Math.floor(number) && Math.abs(number) < 1e15) {
            written.append((long) number);
        } else {
            written.append(number);
        }
    }

    private static void appendMap(StringBuilder written, java.util.Map<?, ?> map) {
        written.append('{');
        boolean first = true;
        for (java.util.Map.Entry<?, ?> entry : map.entrySet()) {
            if (!first) {
                written.append(',');
            }
            first = false;
            appendText(written, String.valueOf(entry.getKey()));
            written.append(':');
            append(written, entry.getValue());
        }
        written.append('}');
    }

    private static void appendItems(StringBuilder written, Iterable<?> items) {
        written.append('[');
        boolean first = true;
        for (Object item : items) {
            if (!first) {
                written.append(',');
            }
            first = false;
            append(written, item);
        }
        written.append(']');
    }

    private static void appendText(StringBuilder written, String text) {
        written.append('"');
        for (int index = 0; index < text.length(); index++) {
            char character = text.charAt(index);
            switch (character) {
                case '"' -> written.append("\\\"");
                case '\\' -> written.append("\\\\");
                case '\n' -> written.append("\\n");
                case '\r' -> written.append("\\r");
                case '\t' -> written.append("\\t");
                case '\b' -> written.append("\\b");
                case '\f' -> written.append("\\f");
                default -> appendCharacter(written, character);
            }
        }
        written.append('"');
    }

    private static void appendCharacter(StringBuilder written, char character) {
        if (character >= ' ') {
            written.append(character);
            return;
        }
        String hex = Integer.toHexString(character);
        written.append("\\u");
        written.append("0000", 0, 4 - hex.length());
        written.append(hex);
    }
}
