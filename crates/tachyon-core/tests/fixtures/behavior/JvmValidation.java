class ValidationChecks {
    static void check(boolean valid) {
        if (!valid) {
            throw new AssertionError("validation contract");
        }
    }
    static void fails(String path, Object value) {
        try {
            Behaviors.validate("Input", value);
            throw new AssertionError("expected validation");
        } catch (Behaviors.Failure failure) {
            check(failure.getMessage().equals("validation") && failure.path.equals(path));
        }
    }
    static java.util.Map<String, Object> validInput() {
        return new java.util.HashMap<>(java.util.Map.of("id", "one", "name", "é", "price", 5));
    }
    static void fieldFails(String field, Object value, String rule) {
        java.util.Map<String, Object> input = validInput();
        input.put(field, value);
        fails(field + "." + rule, input);
    }
    static void run() {
        checkResultIsCopied();
        checkRequiredId();
        checkName();
        checkNumbersAndSizes();
        checkMissingOptionalValues();
        checkInvalidJson();
        System.out.println("Java DTO validation boundary checks passed");
    }
    static void checkResultIsCopied() {
        java.util.Map<String, Object> input = validInput();
        input.put("unknown", true);
        java.util.Map<String, Object> result = Behaviors.validate("Input", input);
        input.put("name", "changed");
        check(result.get("name").equals("é") && result.get("unknown").equals(true));
    }
    static void checkRequiredId() {
        fieldFails("id", null, "notNull");
        java.util.Map<String, Object> missing = validInput();
        missing.remove("id");
        fails("id.notNull", missing);
    }
    static void checkName() {
        fieldFails("name", null, "notBlank");
        fieldFails("name", 1, "notBlank");
        fieldFails("name", "", "notBlank");
        for (int code : new int[]{9, 10, 11, 12, 13, 32, 133, 160, 5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200, 8201, 8202, 8232, 8233, 8239, 8287, 12288}) {
            fieldFails("name", new String(Character.toChars(code)), "notBlank");
        }
        fieldFails("name", "a", "size");
        fieldFails("name", "ééééé", "size");
        java.util.Map<String, Object> invisible = validInput();
        invisible.put("name", "\u200b");
        Behaviors.validate("Input", invisible);
    }
    static void checkNumbersAndSizes() {
        fieldFails("price", "5", "min");
        fieldFails("price", -1, "min");
        fieldFails("price", 11, "max");
        fieldFails("optional", 1, "size");
        fieldFails("optional", java.util.List.of(1, 2, 3), "size");
        fieldFails("optional", java.util.Map.of("a", 1, "b", 2, "c", 3), "size");
    }
    static void checkMissingOptionalValues() {
        java.util.Map<String, Object> ignored = validInput();
        ignored.put("price", null);
        ignored.put("optional", null);
        Behaviors.validate("Input", ignored);
        ignored.remove("price");
        ignored.remove("optional");
        Behaviors.validate("Input", ignored);
    }
    static void checkInvalidJson() {
        fails("$.object", java.util.List.of());
        fails("$.json", new Object());
        java.util.Map<String, Object> notANumber = validInput();
        notANumber.put("price", Double.NaN);
        fails("$.json", notANumber);
        java.util.Map<String, Object> oversized = validInput();
        oversized.put("unknown", "x".repeat(1048576));
        fails("$.json", oversized);
        java.util.List<Object> cycle = new java.util.ArrayList<>();
        cycle.add(cycle);
        fails("$.json", cycle);
    }
}
