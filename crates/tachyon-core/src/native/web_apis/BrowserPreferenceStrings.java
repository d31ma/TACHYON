final class BrowserPreferenceStrings implements PreferenceStrings {
    private final TacPreferenceScope scope;

    BrowserPreferenceStrings(TacPreferenceScope scope) {
        this.scope = scope;
    }

    public String read(String key) {
        return text(call("read", key));
    }

    public boolean write(String key, String value) {
        return Boolean.TRUE.equals(call("write", key, value));
    }

    private Object call(String operation, Object... arguments) {
        try {
            scope.check();
            return TacWebHost.call("preferenceStrings", operation, arguments);
        } catch (IllegalStateException unavailable) {
            return null;
        }
    }

    private String text(Object answer) {
        return answer instanceof String value ? value : null;
    }
}
