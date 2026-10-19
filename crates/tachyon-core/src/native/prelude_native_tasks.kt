
private fun tacTaskHost(): Boolean = false

@Suppress("UNUSED_PARAMETER")
private fun tacBeginTask(activation: TacActivation, task: TacTask, arguments: List<Any?>): String {
    val finished = TacFinishedTask()
    val work: suspend () -> Any? = { task.invoke(arguments) }
    work.startCoroutine(finished)
    return finished.answer ?: TacFinishedTask.WAITING
}

@Suppress("UNUSED_PARAMETER")
private fun tacCancelTasks(owner: String?) {}

private class TacFinishedTask : kotlin.coroutines.Continuation<Any?> {
    override val context: kotlin.coroutines.CoroutineContext = kotlin.coroutines.EmptyCoroutineContext
    var answer: String? = null

    override fun resumeWith(result: Result<Any?>) {
        answer = result.fold({ "{\"value\":${tacEncode(tacPlain(it))}}" }, { tacFailureAnswer(it) })
    }

    companion object {
        const val WAITING =
            "{\"error\":\"This suspend method waits for the browser, and this native service runs without one.\",\"code\":\"HTTP_MISSING\"}"
    }
}
