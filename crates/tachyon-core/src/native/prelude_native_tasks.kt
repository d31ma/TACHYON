
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

private object TacPageSocket {
    private const val MOST_BYTES = 65536

    fun send(client: String, message: Any) {
        val text = try { tacEncode(tacPlain(message)) } catch (_: RuntimeException) { null }
        if (text == null || text.encodeToByteArray().size > MOST_BYTES) {
            throw TacSocketFailure("HTTP_BODY", "A socket message is JSON of at most 64 KiB.")
        }
        val owner = TacNativeState.executing
        if (owner == null || owner.disposed) throw TacSocketFailure("HTTP_REQUEST", "The socket message was not sent.")
        tacSend("{\"route\":${tacEncode(owner.route)},\"service\":${tacEncode(owner.service)},\"owner\":${tacEncode(owner.owner)},\"send\":${tacEncode(client)},\"value\":$text}")
    }
}

@Suppress("RedundantSuspendModifier")
private suspend fun behaviorSocketSend(client: String, message: Any) { TacPageSocket.send(client, message) }
