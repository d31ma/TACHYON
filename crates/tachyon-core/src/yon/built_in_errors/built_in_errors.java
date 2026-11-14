
final class BrokenRule {
    private final String field;
    private final String rule;

    BrokenRule(String field, String rule) {
        this.field = field;
        this.rule = rule;
    }

    public String getField() { return field; }

    public String getRule() { return rule; }
}

abstract class YonBuiltInError extends RuntimeException {
    private final int status;
    private final String reference;

    YonBuiltInError(String name, int status, String reference) {
        super(name, null, false, false);
        this.status = status;
        this.reference = reference;
    }

    public int getStatus() { return status; }

    public String getReference() { return reference; }
}

final class NotSignedIn extends YonBuiltInError {
    NotSignedIn(String reference) { super("NotSignedIn", 401, reference); }
}

final class RoleMissing extends YonBuiltInError {
    RoleMissing(String reference) { super("RoleMissing", 403, reference); }
}

final class IdentityCheckFailed extends YonBuiltInError {
    IdentityCheckFailed(String reference) { super("IdentityCheckFailed", 500, reference); }
}

final class RequestRejected extends YonBuiltInError {
    private final String part;
    private final String field;
    private final String message;

    RequestRejected(String reference, String part, String field, String message) {
        super("RequestRejected", 400, reference);
        this.part = part;
        this.field = field;
        this.message = message;
    }

    public String getPart() { return part; }

    public String getField() { return field; }

    @Override
    public String getMessage() { return message; }
}

final class BodyTooLarge extends YonBuiltInError {
    private final long limitBytes;

    BodyTooLarge(String reference, long limitBytes) {
        super("BodyTooLarge", 413, reference);
        this.limitBytes = limitBytes;
    }

    public long getLimitBytes() { return limitBytes; }
}

final class CheckRejected extends YonBuiltInError {
    private final java.util.List<BrokenRule> violations;
    private final boolean truncated;

    CheckRejected(String reference, java.util.List<BrokenRule> violations, boolean truncated) {
        super("CheckRejected", 422, reference);
        this.violations = java.util.List.copyOf(violations);
        this.truncated = truncated;
    }

    public java.util.List<BrokenRule> getViolations() { return violations; }

    public boolean isTruncated() { return truncated; }
}

final class UnexpectedError extends YonBuiltInError {
    UnexpectedError(String reference) { super("UnexpectedError", 500, reference); }
}

final class __YonBuiltInErrors {
    private __YonBuiltInErrors() {}

    static RuntimeException create(Object value) {
        java.util.Map<?, ?> fields = asFields(value);
        String reference = text(fields, "reference");
        return switch (text(fields, "type")) {
            case "NotSignedIn" -> new NotSignedIn(reference);
            case "RoleMissing" -> new RoleMissing(reference);
            case "IdentityCheckFailed" -> new IdentityCheckFailed(reference);
            case "RequestRejected" -> new RequestRejected(reference, text(fields, "part"), text(fields, "field"), text(fields, "message"));
            case "BodyTooLarge" -> new BodyTooLarge(reference, fields.get("limitBytes") instanceof Number number ? number.longValue() : 0L);
            case "CheckRejected" -> new CheckRejected(reference, rules(fields.get("violations")), Boolean.TRUE.equals(fields.get("truncated")));
            case "UnexpectedError" -> new UnexpectedError(reference);
            default -> throw new IllegalStateException("Unknown built-in error.");
        };
    }

    private static java.util.Map<?, ?> asFields(Object value) {
        return value instanceof java.util.Map<?, ?> map ? map : java.util.Map.of();
    }

    private static String text(java.util.Map<?, ?> fields, String name) {
        Object value = fields.get(name);
        return value == null ? "" : String.valueOf(value);
    }

    private static java.util.List<BrokenRule> rules(Object value) {
        java.util.List<BrokenRule> rules = new java.util.ArrayList<>();
        java.util.List<?> entries = value instanceof java.util.List<?> list ? list : java.util.List.of();
        for (Object entry : entries) {
            java.util.Map<?, ?> rule = asFields(entry);
            rules.add(new BrokenRule(text(rule, "field"), text(rule, "rule")));
        }
        return rules;
    }
}
