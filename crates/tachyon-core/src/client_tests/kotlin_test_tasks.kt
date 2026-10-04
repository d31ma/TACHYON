object BehaviorTasks {
    private const val STILL_WAITING = "The test case still waits. A test adapter must answer at once."

    fun finish(work: suspend () -> Unit) {
        var outcome: Result<Unit>? = null
        work.startCoroutine(kotlin.coroutines.Continuation(kotlin.coroutines.EmptyCoroutineContext) { outcome = it })
        checkNotNull(outcome) { STILL_WAITING }.getOrThrow()
    }
}
