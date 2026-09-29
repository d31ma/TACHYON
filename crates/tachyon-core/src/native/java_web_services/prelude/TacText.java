final class TacText {
    private static final int LONGEST_PROFILE = 64;
    private static final int LONGEST_SEGMENT = 256;
    private static final int LONGEST_HEADER = 128;
    private static final String SEGMENT_MARKS = "._~-";
    private static final String HEADER_MARKS = "!#$%&'*+.^_`|~-";
    private static final String HEX_DIGITS = "0123456789abcdef";
    private static final int HEX_WIDTH = 4;
    private static final int HEX_BASE = 16;

    private TacText() {
    }

    static boolean profileName(String text) {
        if (text.isEmpty() || text.length() > LONGEST_PROFILE || !startsName(text.charAt(0))) {
            return false;
        }
        for (int index = 1; index < text.length(); index++) {
            if (!startsName(text.charAt(index)) && !isDigit(text.charAt(index))) {
                return false;
            }
        }
        return true;
    }

    static boolean pathSegment(String text) {
        return allOf(text, LONGEST_SEGMENT, SEGMENT_MARKS);
    }

    static boolean headerName(String text) {
        return allOf(text, LONGEST_HEADER, HEADER_MARKS);
    }

    static boolean settingName(String text) {
        if (text.isEmpty() || !isUpper(text.charAt(0))) {
            return false;
        }
        for (int index = 1; index < text.length(); index++) {
            char character = text.charAt(index);
            if (!isUpper(character) && !isDigit(character) && character != '_') {
                return false;
            }
        }
        return true;
    }

    static boolean lowerWord(String text) {
        if (text.isEmpty() || !isLower(text.charAt(0))) {
            return false;
        }
        for (int index = 1; index < text.length(); index++) {
            if (!isLower(text.charAt(index)) && !isDigit(text.charAt(index))) {
                return false;
            }
        }
        return true;
    }

    static boolean integerText(String text) {
        return wholeEnd(text, minusEnd(text)) == text.length();
    }

    static boolean numberText(String text) {
        int end = wholeEnd(text, minusEnd(text));
        if (end < 0) {
            return false;
        }
        if (end < text.length() && text.charAt(end) == '.') {
            end = digitsEnd(text, end + 1);
        }
        if (end > 0 && end < text.length() && (text.charAt(end) == 'e' || text.charAt(end) == 'E')) {
            end = digitsEnd(text, signEnd(text, end + 1));
        }
        return end == text.length();
    }

    static String[] split(String text, char separator) {
        java.util.List<String> parts = new java.util.ArrayList<>();
        int start = 0;
        int found = text.indexOf(separator);
        while (found >= 0) {
            parts.add(text.substring(start, found));
            start = found + 1;
            found = text.indexOf(separator, start);
        }
        parts.add(text.substring(start));
        return parts.toArray(new String[0]);
    }

    static String unicodeEscape(int character) {
        StringBuilder escaped = new StringBuilder("\\u");
        for (int shift = (HEX_WIDTH - 1) * 4; shift >= 0; shift -= 4) {
            escaped.append(HEX_DIGITS.charAt((character >> shift) % HEX_BASE));
        }
        return escaped.toString();
    }

    private static boolean allOf(String text, int longest, String marks) {
        if (text.isEmpty() || text.length() > longest) {
            return false;
        }
        for (int index = 0; index < text.length(); index++) {
            char character = text.charAt(index);
            if (!isLetter(character) && !isDigit(character) && marks.indexOf(character) < 0) {
                return false;
            }
        }
        return true;
    }

    private static int minusEnd(String text) {
        return text.startsWith("-") ? 1 : 0;
    }

    private static int signEnd(String text, int start) {
        if (start < 0 || start >= text.length()) {
            return start;
        }
        char character = text.charAt(start);
        return character == '-' || character == '+' ? start + 1 : start;
    }

    private static int wholeEnd(String text, int start) {
        if (start < text.length() && text.charAt(start) == '0') {
            return start + 1;
        }
        if (start >= text.length() || !isDigit(text.charAt(start))) {
            return -1;
        }
        return digitsEnd(text, start);
    }

    private static int digitsEnd(String text, int start) {
        if (start < 0 || start >= text.length() || !isDigit(text.charAt(start))) {
            return -1;
        }
        int end = start;
        while (end < text.length() && isDigit(text.charAt(end))) {
            end++;
        }
        return end;
    }

    private static boolean startsName(char character) {
        return isLetter(character) || character == '_' || character == '$';
    }

    private static boolean isLetter(char character) {
        return isUpper(character) || isLower(character);
    }

    private static boolean isUpper(char character) {
        return character >= 'A' && character <= 'Z';
    }

    private static boolean isLower(char character) {
        return character >= 'a' && character <= 'z';
    }

    private static boolean isDigit(char character) {
        return character >= '0' && character <= '9';
    }
}
