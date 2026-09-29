
final class Assert {
    private static final String NOTHING_THROWN = "Expected an error to be thrown, but nothing was thrown.";
    private static final String MESSAGE_SEPARATOR = " ==> ";

    private Assert() {}

    interface Action {
        void run() throws Throwable;
    }

    static void equals(Object expected, Object actual) {
        equals(expected, actual, null);
    }

    static void equals(Object expected, Object actual, String message) {
        String failure = "expected: <" + expected + "> but was: <" + actual + ">";
        confirm(same(expected, actual), failure, message);
    }

    static void isTrue(boolean condition) {
        isTrue(condition, null);
    }

    static void isTrue(boolean condition, String message) {
        confirm(condition, "expected: <true> but was: <false>", message);
    }

    static Throwable throwsError(Action action) {
        return throwsError(action, null);
    }

    static Throwable throwsError(Action action, String message) {
        Throwable thrown = thrownBy(action);
        confirm(thrown != null, NOTHING_THROWN, message);
        return thrown;
    }

    private static Throwable thrownBy(Action action) {
        try {
            action.run();
            return null;
        } catch (Throwable error) {
            return error;
        }
    }

    private static boolean same(Object expected, Object actual) {
        if (isWhole(expected) && isWhole(actual)) {
            return ((Number) expected).longValue() == ((Number) actual).longValue();
        }
        return java.util.Objects.equals(expected, actual);
    }

    private static boolean isWhole(Object value) {
        return value instanceof Byte || value instanceof Short || value instanceof Integer || value instanceof Long;
    }

    private static void confirm(boolean passed, String failure, String message) {
        if (!passed) {
            throw new AssertionError(text(failure, message));
        }
    }

    private static String text(String failure, String message) {
        if (message == null) {
            return failure;
        }
        return message + MESSAGE_SEPARATOR + failure;
    }
}
