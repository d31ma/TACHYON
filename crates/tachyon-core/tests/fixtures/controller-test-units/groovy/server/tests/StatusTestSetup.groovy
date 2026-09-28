@TestSetup
class StatusTestSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration([:], [:], [:], null, null)
    }
}
