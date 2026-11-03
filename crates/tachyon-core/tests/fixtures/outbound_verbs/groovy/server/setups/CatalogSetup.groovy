@Setup
class CatalogSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(
            java.util.Map.of(),
            java.util.Map.of("catalog", { request, current -> CatalogEchoDelegate.echo(request) } as Behaviors.HttpAdapter),
            java.util.Map.of(), null)
    }
}
