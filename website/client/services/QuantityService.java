@Service
class QuantityService {
    private static final int ONE = 1;
    private static final String SPACE = " ";
    private static final String PLURAL = "s";

    String describe(int count, String noun) {
        return count + SPACE + nounFor(count, noun);
    }

    private String nounFor(int count, String noun) {
        if (count == ONE) {
            return noun;
        }
        return noun + PLURAL;
    }
}
