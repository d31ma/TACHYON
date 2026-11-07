object QueryChecks {
    private val entity = Behaviors.Entity("Product", "data", "products", "id")
    private var result: Any? = listOf(mapOf("id" to "one"))
    private var calls = 0
    private var authorized = 0
    private var allow = true
    private var unavailable = false
    private var command = emptyMap<String, Any?>()
    private fun fails(code: String, call: () -> Unit) {
        try {
            call()
            error("expected $code")
        } catch (failure: Behaviors.Failure) {
            check(failure.code == code)
        }
    }
    private fun authorizeQuery(request: Map<String, Any?>, execution: Behaviors.Execution): Boolean {
        execution.check()
        authorized++
        check(request["operation"] == "query")
        return allow
    }
    private fun executeQuery(request: Map<String, Any?>, execution: Behaviors.Execution): Any? {
        execution.check()
        check(authorized > calls)
        calls++
        command = request
        if (unavailable) {
            error("private database details")
        }
        return result
    }
    private fun context(signal: Behaviors.Signal, authorize: Boolean): Behaviors.Context {
        var authorization: Behaviors.Authorization? = null
        if (authorize) {
            authorization = Behaviors.Authorization { request, execution -> authorizeQuery(request, execution) }
        }
        val options = Behaviors.Options("query-tenant", signal)
        val entities = mapOf("data" to Behaviors.EntityAdapter { request, execution -> executeQuery(request, execution) })
        return Behaviors.Context(options, Behaviors.Configuration(entities = entities, authorizeEntity = authorization))
    }
    private fun query(context: Behaviors.Context, kind: String, where: Array<Array<Any?>> = emptyArray()): Any? {
        return context.query(entity, kind, arrayOf("id", "status"), where, emptyArray(), mapOf("size" to 2), 20, 100)
    }
    fun run() {
        val signal = Behaviors.Signal()
        val context = context(signal, true)
        checkFindCommand(context)
        checkRejectedQueries(context)
        checkAcceptedBindings(context)
        checkResultShapes(context)
        checkUnavailableAdapter(context)
        checkCancellation(signal, context)
        println("Kotlin declarative query boundary checks passed")
    }
    private fun checkFindCommand(context: Behaviors.Context) {
        check(query(context, "find", arrayOf(arrayOf("status", "eq", "active"), arrayOf("id", "isNotNull"))) == result)
        check(command["table"] == "products" && command["idField"] == "id")
        val descriptor = command["query"] as Map<*, *>
        check((descriptor["page"] as Map<*, *>)["size"] == 2L)
    }
    private fun checkRejectedQueries(context: Behaviors.Context) {
        val callsBefore = calls
        allow = false
        fails("authorization") { query(context, "find") }
        allow = true
        fails("authorization") { query(context(Behaviors.Signal(), false), "find") }
        check(calls == callsBefore)
        fails("query-binding") { query(context, "find", arrayOf(arrayOf("id", "eq", emptyMap<String, Any?>()))) }
        fails("query-binding") { query(context, "find", arrayOf(arrayOf("id", "in", List(101) { "one" }))) }
        fails("json-number") { query(context, "find", arrayOf(arrayOf("id", "eq", Double.NaN))) }
        fails("json-size") { query(context, "find", arrayOf(arrayOf("id", "eq", "x".repeat(40000)), arrayOf("status", "eq", "x".repeat(40000)))) }
        fails("pagination") { context.query(entity, "find", arrayOf("id"), emptyArray(), arrayOf(arrayOf("id", "asc")), mapOf("sort" to listOf("id")), 20, 100) }
        check(calls == callsBefore)
    }
    private fun checkAcceptedBindings(context: Behaviors.Context) {
        query(context, "find", arrayOf(arrayOf("id", "eq", "x".repeat(65480))))
        query(context, "find", arrayOf(arrayOf("id", "in", listOf(null, "one", 2, false))))
        for (operator in listOf("eq", "ne", "lt", "lte", "gt", "gte")) {
            query(context, "find", arrayOf(arrayOf("id", operator, null)))
        }
    }
    private fun checkResultShapes(context: Behaviors.Context) {
        result = 7
        check(query(context, "count") == 7L)
        check("page" !in (command["query"] as Map<*, *>))
        result = true
        check(query(context, "exists") == true)
        result = -1
        fails("query-result") { query(context, "count") }
        result = "true"
        fails("query-result") { query(context, "exists") }
        result = listOf("not-row")
        fails("query-result") { query(context, "find") }
        result = List(3) { emptyMap<String, Any?>() }
        fails("query-result") { query(context, "find") }
        result = listOf(mapOf("id" to "x".repeat(1048576)))
        fails("json-size") { query(context, "find") }
    }
    private fun checkUnavailableAdapter(context: Behaviors.Context) {
        unavailable = true
        fails("entity-adapter") { query(context, "find") }
        unavailable = false
    }
    private fun checkCancellation(signal: Behaviors.Signal, context: Behaviors.Context) {
        val callsBefore = calls
        signal.abort()
        fails("aborted") { query(context, "find") }
        check(calls == callsBefore)
        val entities = mapOf("data" to Behaviors.EntityAdapter { _, _ -> error("executed after abort") })
        val abortWhileAuthorizing = Behaviors.Authorization { _, execution ->
            execution.signal.abort()
            true
        }
        val options = Behaviors.Options("cancelled-query", Behaviors.Signal())
        val configuration = Behaviors.Configuration(entities = entities, authorizeEntity = abortWhileAuthorizing)
        val cancelled = Behaviors.Context(options, configuration)
        fails("aborted") { query(cancelled, "find") }
    }
}
