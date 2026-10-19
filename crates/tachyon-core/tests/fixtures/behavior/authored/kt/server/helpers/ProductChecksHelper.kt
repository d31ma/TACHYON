@Helper
class ProductChecksHelper {
    fun run(): Any? {
        val context = Behaviors.create("RuntimeAdaptersSetup", Behaviors.Options(TENANT, Behaviors.Signal()))
        context.use {
            checkConfiguration()
            val catalog = CatalogService()
            catalog.behaviors = context
            val client = CatalogClient()
            client.behaviors = context
            checkValidation(catalog, client)
            val page = mapOf<String, Any?>(PAGE to 2, "size" to PAGE_SIZE)
            val first = catalog.list(page)
            val cached = catalog.list(page)
            catalog.remove("ignored", page)
            val fresh = catalog.list(page)
            checkCachePut(catalog, page)
            checkCacheKey(catalog)
            checkDefaultPage(catalog)
            checkDeclaredExchange(client)
            checkResilience(context)
            val queries = ProductQueriesStore()
            queries.behaviors = context
            checkDerivedQueries(queries)
            checkRepository(queries)
            checkExplicitQuery(queries, page)
            checkMapping(context)
            checkEntityAuthorization(queries)
            val repository = context.repository("ProductEntity")
            return mapOf(
                "first" to first, "cached" to cached, "fresh" to fresh,
                "remote" to client.remote(1L), "saved" to repository.save(mapOf(ID to ONE)),
                "found" to repository.find(), "deleted" to repository.delete(ONE))
        }
    }
    private fun expectFailure(code: String, missingMessage: String, call: () -> Unit): Behaviors.Failure {
        try {
            call()
        } catch (failure: Behaviors.Failure) {
            check(failure.code == code)
            return failure
        }
        error(missingMessage)
    }
    private fun checkConfiguration() {
        val settings = Behaviors.properties("Settings", mapOf("YON_APP_TITLE" to PRODUCTS, "YON_APP_COUNT" to "12"))
        check(settings[TITLE] == PRODUCTS && settings["count"] == COUNT && settings["enabled"] == false && settings["ratio"] == RATIO)
        val configuredBefore = RuntimeAdaptersSetup.configured
        expectFailure("inactive-configuration", "expected inactive configuration") { Behaviors.create(SELECTED_SETUP, Behaviors.Options(TENANT, Behaviors.Signal())) }
        check(RuntimeAdaptersSetup.configured == configuredBefore)
        val selected = Behaviors.Options(TENANT, Behaviors.Signal(), profiles = listOf("development"), properties = mapOf("YON_ENABLED" to "true"))
        Behaviors.create(SELECTED_SETUP, selected).close()
    }
    private fun checkValidation(catalog: CatalogService, client: CatalogClient) {
        val validInput = mapOf(ID to ONE, NAME to "é", "price" to PRICE, "unknown" to true)
        val validator = Validator()
        check(validator.accept(validInput) == validInput && validator.calls == 1)
        val authorizedBefore = RuntimeAdaptersSetup.authorized
        val failure = expectFailure(VALIDATION, "expected validation") { catalog.checked(mapOf(ID to ONE, NAME to "secret invalid input")) }
        check(failure.path == "name.size")
        check(RuntimeAdaptersSetup.authorized == authorizedBefore && catalog.validCalls == 0)
        check(catalog.checked(validInput) == catalog.checked(validInput) && catalog.validCalls == 1)
        expectFailure(VALIDATION, "expected validation before HTTP") { client.checkedRemote(emptyMap<String, Any?>()) }
        check((client.checkedRemote(validInput) as Map<*, *>)[PATH] == "/validated")
    }
    private fun checkCachePut(catalog: CatalogService, page: Map<String, Any?>) {
        val updated = catalog.update("replacement", page)
        check(updated == catalog.list(page) && catalog.calls == EXPECTED_CALLS && catalog.mutations == EXPECTED_CALLS)
        RuntimeAdaptersSetup.allow = false
        expectFailure(AUTHORIZATION, "expected authorization") { catalog.update("denied", page) }
        RuntimeAdaptersSetup.allow = true
        check(catalog.mutations == EXPECTED_CALLS)
        expectFailure(JSON_VALUE, "expected bounded JSON") { catalog.invalid(page) }
        check(updated == catalog.list(page))
    }
    private fun checkCacheKey(catalog: CatalogService) {
        catalog.price(SKU, "first")
        catalog.price(SKU, "second")
        check(catalog.priceCalls == 1)
        catalog.price("sku-2", "first")
        check(catalog.priceCalls == EXPECTED_CALLS)
    }
    private fun checkDefaultPage(catalog: CatalogService) {
        val defaultUpdated = catalog.update("default", emptyMap())
        check(defaultUpdated == catalog.list(emptyMap()))
    }
    private fun checkDeclaredExchange(catalog: CatalogClient) {
        val declared = catalog.search(PAGE_NUMBER) as Map<*, *>
        if ((declared[QUERY] as Map<*, *>)[PAGE] != PAGE_NUMBER) {
            throw AssertionError("declared HttpExchange")
        }
        if ((catalog.find(PRODUCT_ID) as Map<*, *>)[PATH] != "/products/7") {
            throw AssertionError("path variable")
        }
    }
    private fun checkResilience(context: Behaviors.Context) {
        val resilient = Resilient(context)
        resilient.flaky()
        if (resilient.attempts != ATTEMPTS) {
            throw AssertionError("Retryable contract")
        }
        if (resilient.bounded(OK) != OK) {
            throw AssertionError("ConcurrencyLimit contract")
        }
    }
    private fun firstQuery(rows: List<*>): Map<*, *> {
        val firstRow = rows[0] as Map<*, *>
        return firstRow[QUERY] as Map<*, *>
    }
    private fun checkDerivedQueries(queries: ProductQueriesStore) {
        val rows = queries.findByStatus(ACTIVE) as List<*>
        val query = firstQuery(rows)
        val firstPage = (query[PAGE] as Map<*, *>)[OFFSET] == 0L
        check(firstPage && ((query["where"] as List<*>)[0] as Map<*, *>)["value"] == ACTIVE)
        check(queries.countByStatus(ACTIVE) == 1L && queries.existsById(ONE) == true)
    }
    private fun checkRepository(queries: ProductQueriesStore) {
        check((queries.findById(ONE) as Map<*, *>)[ID] == ONE)
        check((queries.findAll(mapOf()) as List<*>).size == 1)
        check((queries.save(mapOf(ID to ONE)) as Map<*, *>)["operation"] == "save")
        check(queries.deleteById(ONE) == 1L && queries.deleteByStatus(ACTIVE) == 1L)
    }
    private fun checkExplicitQuery(queries: ProductQueriesStore, page: Map<String, Any?>) {
        val explicit = queries.available(listOf(ACTIVE), page) as List<*>
        val explicitQuery = firstQuery(explicit)
        check((explicitQuery[PAGE] as Map<*, *>)[OFFSET] == EXPLICIT_OFFSET)
    }
    private fun checkMapping(context: Behaviors.Context) {
        val summaries = ProductSummaries()
        val product = mapOf("detail" to mapOf(ID to PRODUCT_ID), NAME to ANVIL, "extra" to "dropped")
        val summary = summaries.toSummary(product) as Map<*, *>
        check(summary.size == SUMMARY_FIELDS && summary["productId"] == PRODUCT_ID && summary[TITLE] == ANVIL
            && summary["absent"] == null && summary["kind"] == "product")
        expectFailure(JSON_VALUE, "a mapping reads one JSON object argument") { summaries.toSummary(listOf<Any?>()) }
    }
    private fun checkEntityAuthorization(queries: ProductQueriesStore) {
        RuntimeAdaptersSetup.allow = false
        expectFailure(AUTHORIZATION, "expected entity authorization") { queries.existsById(ONE) }
        RuntimeAdaptersSetup.allow = true
    }

    companion object {
        private const val PAGE_SIZE = 5
        private const val PRICE = 5
        private const val COUNT = 12L
        private const val RATIO = 1.5
        private const val EXPECTED_CALLS = 2
        private const val PAGE_NUMBER = 2L
        private const val ATTEMPTS = 3
        private const val EXPLICIT_OFFSET = 10L
        private const val PRODUCT_ID = 7L
        private const val SUMMARY_FIELDS = 4
        private const val TENANT = "tenant"
        private const val PRODUCTS = "Products"
        private const val SELECTED_SETUP = "SelectedAdaptersSetup"
        private const val ACTIVE = "active"
        private const val AUTHORIZATION = "authorization"
        private const val ID = "id"
        private const val JSON_VALUE = "json-value"
        private const val NAME = "name"
        private const val OFFSET = "offset"
        private const val OK = "ok"
        private const val ONE = "one"
        private const val PAGE = "page"
        private const val PATH = "path"
        private const val QUERY = "query"
        private const val TITLE = "title"
        private const val VALIDATION = "validation"
        private const val ANVIL = "Anvil"
        private const val SKU = "sku-1"
    }
}
