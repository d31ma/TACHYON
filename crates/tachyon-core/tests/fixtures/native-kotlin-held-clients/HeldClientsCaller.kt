package app.held

object HeldClientsCaller {
    const val CHAT_OWNER = "00000000-0000-4000-8000-000000000001"
    const val STREAM_OWNER = "00000000-0000-4000-8000-000000000002"

    fun ask(owner: String, service: String, rest: String): String =
        tacNativeInvoke("{\"route\":\"/\",\"service\":\"$service\",\"owner\":\"$owner\",$rest}")

    fun described(answer: String): org.json.JSONObject = org.json.JSONObject(answer).getJSONObject("value")

    fun first(list: org.json.JSONArray, key: String?): String =
        if (key == null) list.getString(0) else list.getJSONObject(0).getString(key)
}

fun main() {
    val signals = mutableListOf<String>()
    TacBridge.emit = TacEmit { signals.add(it) }
    val chatInit = HeldClientsCaller.ask(HeldClientsCaller.CHAT_OWNER, "chat", "\"op\":\"init\"")
    println("chat init $chatInit")
    val chat = HeldClientsCaller.described(chatInit)
    val heard = HeldClientsCaller.first(chat.getJSONArray("listeners"), "key")
    println("chat event " + HeldClientsCaller.ask(HeldClientsCaller.CHAT_OWNER, "chat", "\"op\":\"event\",\"name\":\"$heard\",\"value\":{\"text\":\"from the page\"}"))
    println("chat field " + HeldClientsCaller.ask(HeldClientsCaller.CHAT_OWNER, "chat", "\"op\":\"get\",\"name\":\"" + HeldClientsCaller.first(chat.getJSONArray("fields"), null) + "\""))
    println("signals before send " + signals.size)
    println("chat send " + HeldClientsCaller.ask(HeldClientsCaller.CHAT_OWNER, "chat", "\"op\":\"call\",\"name\":\"sayHello\",\"args\":[]"))
    signals.forEach { println("chat signal $it") }
    val streamInit = HeldClientsCaller.ask(HeldClientsCaller.STREAM_OWNER, "stream", "\"op\":\"init\"")
    println("stream init $streamInit")
    val stream = HeldClientsCaller.described(streamInit)
    val tick = HeldClientsCaller.first(stream.getJSONArray("listeners"), "key")
    println("stream event " + HeldClientsCaller.ask(HeldClientsCaller.STREAM_OWNER, "stream", "\"op\":\"event\",\"name\":\"$tick\",\"value\":{\"sku\":\"A\",\"level\":7}"))
    println("stream field " + HeldClientsCaller.ask(HeldClientsCaller.STREAM_OWNER, "stream", "\"op\":\"get\",\"name\":\"" + HeldClientsCaller.first(stream.getJSONArray("fields"), null) + "\""))
    println("dispose " + tacNativeInvoke("{\"op\":\"host-dispose\"}"))
}
