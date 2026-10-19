@Setup
class CatalogSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration =
            Behaviors.Configuration(http = mapOf("catalog" to Behaviors.HttpAdapter { request, _ -> CatalogEchoHelper.echo(request) }))
    }
}
