@Service
class CatalogService {
    @Needs
    val catalog: CatalogClient
    fun products(): Any? = catalog.all(1L)
}
