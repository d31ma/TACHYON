class SettingsService {
    var title: String = "Catalog"
    var pageSize: Long = 20

    init {
        val publicCatalog = mapOf(
            "title" to "Catalog",
            "page" to mapOf("size" to 20),
            "cache" to mapOf("enabled" to true),
        )
        val published = mapOf("tac" to mapOf("catalog" to publicCatalog))
        val setup = "CatalogSetup"
        val catalog = Behaviors.properties(setup, published)
        val values = Behaviors.values("PublicValues", published)
        title = values["title"] as String
        pageSize = catalog["pageSize"] as Long
        Behaviors.create(setup, Behaviors.Options(
            "public-catalog", Behaviors.Signal(), profiles = listOf("public"), properties = published
        )).close()
    }
}
