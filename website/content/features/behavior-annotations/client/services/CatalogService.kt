@Service
class CatalogService {
    lateinit var behaviors: Behaviors.Context
    var offset: Long = 0

    fun page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) input: Map<String, Any?>): Any? = input["offset"]

    @OnStart
    fun initialize() {
        behaviors = Behaviors.create("ApplicationSetup", Behaviors.Options("public-catalog", Behaviors.Signal()))
        offset = (page(mapOf("page" to 1)) as Number).toLong()
    }
}
