final class TacValues {
    private static final String WRONG_TYPE = "A service argument or field value has the wrong type.";

    private TacValues() {
    }

    static java.util.Map<?, ?> fields(Object value) {
        if (value instanceof java.util.Map<?, ?> fields) {
            return fields;
        }
        throw wrongType();
    }

    static Object field(java.util.Map<?, ?> fields, String name, boolean required) {
        Object value = fields.get(name);
        if (required && value == null) {
            throw wrongType();
        }
        return value;
    }

    static IllegalArgumentException wrongType() {
        return new IllegalArgumentException(WRONG_TYPE);
    }

    static Object argument(java.util.List<Object> arguments, int index) {
        return index < arguments.size() ? arguments.get(index) : null;
    }

    static int integer(Object value) {
        return (int) decimal(value);
    }

    static long whole(Object value) {
        if (value instanceof Integer || value instanceof Long) {
            return ((Number) value).longValue();
        }
        return (long) decimal(value);
    }

    static float single(Object value) {
        return (float) decimal(value);
    }

    static double decimal(Object value) {
        if (value instanceof Number number) {
            return number.doubleValue();
        }
        if (value instanceof Boolean truth) {
            return truth ? 1 : 0;
        }
        try {
            return value == null ? 0 : Double.parseDouble(value.toString());
        } catch (NumberFormatException error) {
            return 0;
        }
    }

    static boolean truth(Object value) {
        if (value instanceof Boolean truth) {
            return truth;
        }
        if (value instanceof Number number) {
            return number.doubleValue() != 0;
        }
        return value instanceof String text && !text.isEmpty();
    }

    static String text(Object value) {
        return value == null ? null : value.toString();
    }

    static Integer optionalInteger(Object value) {
        return value == null ? null : integer(value);
    }

    static Long optionalWhole(Object value) {
        return value == null ? null : whole(value);
    }

    static Float optionalSingle(Object value) {
        return value == null ? null : single(value);
    }

    static Double optionalDecimal(Object value) {
        return value == null ? null : decimal(value);
    }

    static Boolean optionalTruth(Object value) {
        return value == null ? null : truth(value);
    }

    static <T> java.util.List<T> list(Object value, java.util.function.Function<Object, T> item) {
        if (value == null) {
            return null;
        }
        java.util.List<T> items = new java.util.ArrayList<>();
        if (value instanceof java.util.List<?> list) {
            for (Object each : list) {
                items.add(item.apply(each));
            }
        }
        return items;
    }

    static <T> java.util.Map<String, T> map(Object value, java.util.function.Function<Object, T> item) {
        if (value == null) {
            return null;
        }
        java.util.Map<String, T> entries = new java.util.LinkedHashMap<>();
        if (value instanceof java.util.Map<?, ?> map) {
            for (java.util.Map.Entry<?, ?> entry : map.entrySet()) {
                entries.put(String.valueOf(entry.getKey()), item.apply(entry.getValue()));
            }
        }
        return entries;
    }

    static Object same(Object value) {
        return value;
    }
}
