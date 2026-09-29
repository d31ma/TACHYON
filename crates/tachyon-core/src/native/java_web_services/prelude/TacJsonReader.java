final class TacJsonReader {
    private static final int DEEPEST = 64;
    private final String text;
    private int at;

    private TacJsonReader(String text) {
        this.text = text;
    }

    static Object read(String text) {
        TacJsonReader reader = new TacJsonReader(text);
        Object value = reader.value(0);
        reader.space();
        if (reader.at != text.length()) {
            throw new IllegalArgumentException("Unexpected text after the JSON value.");
        }
        return value;
    }

    private void space() {
        while (at < text.length() && " \t\r\n".indexOf(text.charAt(at)) >= 0) {
            at++;
        }
    }

    private void expect(boolean holds, String problem) {
        if (!holds) {
            throw new IllegalArgumentException(problem);
        }
    }

    private Object value(int depth) {
        expect(depth <= DEEPEST, "JSON is nested too deeply.");
        space();
        expect(at < text.length(), "JSON ended early.");
        char first = text.charAt(at);
        if (first == '{') {
            return objectValue(depth);
        }
        if (first == '[') {
            return listValue(depth);
        }
        if (first == '"') {
            return textValue();
        }
        if (text.startsWith("true", at)) {
            return word("true", Boolean.TRUE);
        }
        if (text.startsWith("false", at)) {
            return word("false", Boolean.FALSE);
        }
        if (text.startsWith("null", at)) {
            return word("null", null);
        }
        return numberValue();
    }

    private Object word(String written, Object value) {
        at += written.length();
        return value;
    }

    private java.util.Map<String, Object> objectValue(int depth) {
        java.util.Map<String, Object> entries = new java.util.LinkedHashMap<>();
        at++;
        space();
        if (at < text.length() && text.charAt(at) == '}') {
            at++;
            return entries;
        }
        while (true) {
            space();
            expect(at < text.length() && text.charAt(at) == '"', "A JSON key must be text.");
            String key = textValue();
            space();
            expect(at < text.length() && text.charAt(at) == ':', "A JSON key needs a colon.");
            at++;
            entries.put(key, value(depth + 1));
            space();
            expect(at < text.length(), "JSON ended early.");
            if (text.charAt(at) == '}') {
                at++;
                return entries;
            }
            expect(text.charAt(at) == ',', "JSON entries need commas.");
            at++;
        }
    }

    private java.util.List<Object> listValue(int depth) {
        java.util.List<Object> items = new java.util.ArrayList<>();
        at++;
        space();
        if (at < text.length() && text.charAt(at) == ']') {
            at++;
            return items;
        }
        while (true) {
            items.add(value(depth + 1));
            space();
            expect(at < text.length(), "JSON ended early.");
            if (text.charAt(at) == ']') {
                at++;
                return items;
            }
            expect(text.charAt(at) == ',', "JSON items need commas.");
            at++;
        }
    }

    private String textValue() {
        StringBuilder result = new StringBuilder();
        at++;
        while (true) {
            expect(at < text.length(), "JSON text is not closed.");
            char character = text.charAt(at++);
            if (character == '"') {
                return result.toString();
            }
            result.append(character == '\\' ? escaped() : character);
        }
    }

    private char escaped() {
        expect(at < text.length(), "JSON text is not closed.");
        char code = text.charAt(at++);
        return switch (code) {
            case 'n' -> '\n';
            case 'r' -> '\r';
            case 't' -> '\t';
            case 'b' -> '\b';
            case 'f' -> '\f';
            case 'u' -> characterCode();
            default -> code;
        };
    }

    private char characterCode() {
        expect(at + 4 <= text.length(), "A JSON character code is too short.");
        char character = (char) Integer.parseInt(text.substring(at, at + 4), 16);
        at += 4;
        return character;
    }

    private Object numberValue() {
        int start = at;
        while (at < text.length() && "+-0123456789.eE".indexOf(text.charAt(at)) >= 0) {
            at++;
        }
        String number = text.substring(start, at);
        expect(!number.isEmpty(), "Unknown JSON value.");
        boolean whole = number.indexOf('.') < 0 && number.indexOf('e') < 0 && number.indexOf('E') < 0;
        if (!whole) {
            return Double.parseDouble(number);
        }
        long value = Long.parseLong(number);
        return value == (int) value ? Integer.valueOf((int) value) : Long.valueOf(value);
    }
}
