final class TacMaps {
    private TacMaps() {
    }

    static <K, V> java.util.Map<K, V> copyOf(java.util.Map<? extends K, ? extends V> map) {
        return java.util.Collections.unmodifiableMap(new java.util.LinkedHashMap<>(map));
    }
}
