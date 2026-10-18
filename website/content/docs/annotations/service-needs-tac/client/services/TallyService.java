@Service
class TallyService {
    @Needs
    private final CounterService counter;
    int total = 0;

    void bump() {
        total = counter.increment();
    }
}
