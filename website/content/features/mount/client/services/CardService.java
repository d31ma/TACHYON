@Service
class CardService {
    final String name = "Anvil";
    String status = "waiting";

    @OnStart
    void activate() { status = "active"; }
}
