
private fun tacTaskHost(): Boolean = false

@Suppress("UNUSED_PARAMETER")
private fun tacBeginTask(activation: TacActivation, task: TacTask, arguments: List<Any?>): String =
    "{\"error\":\"This suspend method waits for the browser, and this native service runs without one.\",\"code\":\"HTTP_MISSING\"}"

@Suppress("UNUSED_PARAMETER")
private fun tacCancelTasks(owner: String?) {}
