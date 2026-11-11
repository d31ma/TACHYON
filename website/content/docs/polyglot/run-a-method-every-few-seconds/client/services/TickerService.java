@Service
class TickerService {
    int seconds = 0;

    @Every("1s")
    void count() {
        seconds += 1;
    }
}
