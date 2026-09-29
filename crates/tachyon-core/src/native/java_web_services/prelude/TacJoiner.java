final class TacJoiner {
    private final String separator;
    private final String suffix;
    private final StringBuilder joined;
    private boolean empty = true;

    TacJoiner(CharSequence separator, CharSequence prefix, CharSequence suffix) {
        this.separator = separator.toString();
        this.suffix = suffix.toString();
        this.joined = new StringBuilder(prefix);
    }

    TacJoiner add(CharSequence item) {
        if (!empty) {
            joined.append(separator);
        }
        joined.append(item);
        empty = false;
        return this;
    }

    @Override
    public String toString() {
        return joined + suffix;
    }
}
