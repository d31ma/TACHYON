@Delegate
class ProductChecksDelegate {
    private static final int PAGE_SIZE = 5
    private static final int PRICE = 5
    private static final int EXPECTED_CALLS = 2
    private static final long PAGE_NUMBER = 2L
    private static final int ATTEMPTS = 3
    private static final int SUMMARY_FIELDS = 4
    private static final long PRODUCT_ID = 7L
    private static final String TENANT = "tenant"
    private static final String PRODUCTS = "Products"
    private static final String SELECTED_SETUP = "SelectedAdaptersSetup"
    private static final String ACTIVE = "active"
    private static final String AUTHORIZATION = "authorization"
    private static final String ID = "id"
    private static final String JSON_VALUE = "json-value"
    private static final String NAME = "name"
    private static final String OFFSET = "offset"
    private static final String OK = "ok"
    private static final String ONE = "one"
    private static final String PAGE = "page"
    private static final String PATH = "path"
    private static final String QUERY = "query"
    private static final String TITLE = "title"
    private static final String VALIDATION = "validation"
    private static final String ANVIL = "Anvil"
    private static final String SKU = "sku-1"
    static Object run() {
        try (Behaviors.Context context = Behaviors.create("RuntimeAdaptersSetup", new Behaviors.Options(TENANT, new Behaviors.Signal()))) {
            checkConfiguration()
            CatalogService catalog = new CatalogService()
            catalog.behaviors = context
            CatalogClient client = new CatalogClient()
            client.behaviors = context
            checkValidation(catalog, client)
            java.util.Map<String, Object> page = java.util.Map.of(PAGE, 2, "size", PAGE_SIZE)
            Object first = catalog.list(page)
            Object cached = catalog.list(page)
            catalog.remove("ignored", page)
            Object fresh = catalog.list(page)
            checkCachePut(catalog, page)
            checkCacheKey(catalog)
            checkDefaultPage(catalog)
            checkDeclaredExchange(client)
            checkResilience(context)
            ProductQueriesStore queries = new ProductQueriesStore()
            queries.behaviors = context
            checkDerivedQueries(queries)
            checkRepository(queries)
            checkExplicitQuery(queries, page)
            checkMapping(context)
            checkEntityAuthorization(queries)
            Behaviors.Repository repository = context.repository("ProductEntity")
            return java.util.Map.of(
                "first", first, "cached", cached, "fresh", fresh,
                "remote", client.remote(1L), "saved", repository.save(java.util.Map.of(ID, ONE)),
                "found", repository.find(java.util.Map.of()), "deleted", repository.delete(ONE))
        }
    }
    static void expect(boolean condition, String message) {
        if (!condition) {
            throw new AssertionError(message)
        }
    }
    static Behaviors.Failure expectFailure(String code, String missingMessage, Runnable call) {
        try {
            call.run()
        } catch (Behaviors.Failure failure) {
            if (!failure.getMessage().equals(code)) {
                throw failure
            }
            return failure
        }
        throw new AssertionError(missingMessage)
    }
    static void checkConfiguration() {
        java.util.Map<String, Object> settings = Behaviors.properties("Settings", java.util.Map.of("YON_APP_TITLE", PRODUCTS, "YON_APP_COUNT", "12"))
        boolean settingsMatch = settings.get(TITLE).equals(PRODUCTS) && settings.get("count").equals(12L) && settings.get("enabled").equals(false) && settings.get("ratio").equals(1.5d)
        expect(settingsMatch, "configuration properties")
        int configuredBefore = RuntimeAdaptersSetup.configured
        expectFailure("inactive-configuration", "expected inactive configuration", () -> Behaviors.create(SELECTED_SETUP, new Behaviors.Options(TENANT, new Behaviors.Signal())))
        expect(RuntimeAdaptersSetup.configured == configuredBefore, "configuration ran before selection")
        Behaviors.Options selected = new Behaviors.Options(TENANT, new Behaviors.Signal(), 10000, java.util.List.of("development"), java.util.Map.of("YON_ENABLED", "true"))
        Behaviors.create(SELECTED_SETUP, selected).close()
    }
    static void checkValidation(CatalogService catalog, CatalogClient client) {
        Object validInput = java.util.Map.of(ID, ONE, NAME, "é", "price", PRICE, "unknown", true)
        Validator validator = new Validator()
        expect(validator.accept(validInput).equals(validInput) && validator.calls == 1, "standalone Valid")
        int authorizedBefore = RuntimeAdaptersSetup.authorized
        Behaviors.Failure failure = expectFailure(VALIDATION, "expected validation", () -> catalog.checked(java.util.Map.of(ID, ONE, NAME, "secret invalid input")))
        if (!failure.path.equals("name.size")) {
            throw failure
        }
        expect(RuntimeAdaptersSetup.authorized == authorizedBefore && catalog.validCalls == 0, "invalid input reached cache/body")
        expect(catalog.checked(validInput).equals(catalog.checked(validInput)) && catalog.validCalls == 1, "Valid Cacheable pipeline")
        expectFailure(VALIDATION, "expected validation before HTTP", () -> client.checkedRemote(java.util.Map.of()))
        expect(Behaviors.map(client.checkedRemote(validInput)).get(PATH).equals("/validated"), "Valid HttpExchange pipeline")
    }
    static void checkCachePut(CatalogService catalog, java.util.Map<String, Object> page) {
        Object updated = catalog.update("replacement", page)
        expect(updated.equals(catalog.list(page)) && catalog.calls == EXPECTED_CALLS && catalog.mutations == EXPECTED_CALLS, "CachePut target mapping")
        RuntimeAdaptersSetup.allow = false
        expectFailure(AUTHORIZATION, "expected authorization", () -> catalog.update("denied", page))
        RuntimeAdaptersSetup.allow = true
        expect(catalog.mutations == EXPECTED_CALLS, "mutation before authorization")
        expectFailure(JSON_VALUE, "expected bounded JSON", () -> catalog.invalid(page))
        expect(updated.equals(catalog.list(page)), "invalid result changed cache")
    }
    static void checkCacheKey(CatalogService catalog) {
        catalog.price(SKU, "first")
        catalog.price(SKU, "second")
        expect(catalog.priceCalls == 1, "CacheKey hit ignores the unmarked argument")
        catalog.price("sku-2", "first")
        expect(catalog.priceCalls == EXPECTED_CALLS, "CacheKey miss on a different key")
    }
    static void checkDefaultPage(CatalogService catalog) {
        Object defaultUpdated = catalog.update("default", java.util.Map.of())
        expect(defaultUpdated.equals(catalog.list(java.util.Map.of())), "raw mutation Pageable mapping")
    }
    static void checkDeclaredExchange(CatalogClient catalog) {
        java.util.Map<String, Object> declared = Behaviors.map(catalog.search(PAGE_NUMBER))
        expect(Behaviors.map(declared.get(QUERY)).get(PAGE).equals(PAGE_NUMBER), "declared HttpExchange")
        expect(Behaviors.map(catalog.find(PRODUCT_ID)).get(PATH).equals("/products/7"), "path variable")
    }
    static void checkResilience(Behaviors.Context context) {
        Resilient resilient = new Resilient(context)
        resilient.flaky()
        expect(resilient.attempts == ATTEMPTS, "Retryable contract")
        expect(resilient.bounded(OK).equals(OK), "ConcurrencyLimit contract")
    }
    static java.util.Map<String, Object> firstQuery(Object rows) {
        Object firstRow = ((java.util.List<?>)rows).get(0)
        return Behaviors.map(Behaviors.map(firstRow).get(QUERY))
    }
    static void checkDerivedQueries(ProductQueriesStore queries) {
        Object rows = queries.findByStatus(ACTIVE)
        java.util.Map<String, Object> query = firstQuery(rows)
        boolean firstPage = Behaviors.map(query.get(PAGE)).get(OFFSET).equals(0L)
        expect(firstPage && Behaviors.map(((java.util.List<?>)query.get("where")).get(0)).get("value").equals(ACTIVE), "derived query bindings")
        expect(queries.countByStatus(ACTIVE).equals(1L) && queries.existsById(ONE).equals(true), "derived result shape")
    }
    static void checkRepository(ProductQueriesStore queries) {
        expect(Behaviors.map(queries.findById(ONE)).get(ID).equals(ONE), "repository findById")
        expect(((java.util.List<?>)queries.findAll(java.util.Map.of())).size() == 1, "repository findAll")
        expect(Behaviors.map(queries.save(java.util.Map.of(ID, ONE))).get("operation").equals("save"), "repository save")
        expect(queries.deleteById(ONE).equals(1L) && queries.deleteByStatus(ACTIVE).equals(1L), "repository delete")
    }
    static void checkExplicitQuery(ProductQueriesStore queries, java.util.Map<String, Object> page) {
        Object explicit = queries.available(java.util.List.of(ACTIVE), page)
        java.util.Map<String, Object> explicitQuery = firstQuery(explicit)
        expect(Behaviors.map(explicitQuery.get(PAGE)).get(OFFSET).equals(10L), "explicit query pagination")
    }
    static void checkMapping(Behaviors.Context context) {
        ProductSummaries summaries = new ProductSummaries()
        java.util.Map<String, Object> product = java.util.Map.of("detail", java.util.Map.of(ID, PRODUCT_ID), NAME, ANVIL, "extra", "dropped")
        java.util.Map<String, Object> summary = Behaviors.map(summaries.toSummary(product))
        boolean fieldsMatch = summary.get("productId").equals(PRODUCT_ID) && summary.get(TITLE).equals(ANVIL)
            && summary.get("absent") == null && summary.get("kind").equals("product")
            && summary.size() == SUMMARY_FIELDS
        expect(fieldsMatch, "mapping fields")
        expectFailure(JSON_VALUE, "a mapping reads one JSON object argument", () -> summaries.toSummary(java.util.List.of()))
    }
    static void checkEntityAuthorization(ProductQueriesStore queries) {
        RuntimeAdaptersSetup.allow = false
        expectFailure(AUTHORIZATION, "expected entity authorization", () -> queries.existsById(ONE))
        RuntimeAdaptersSetup.allow = true
    }
}

