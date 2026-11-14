class CodeScanner {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("codeScanner", "supported"))

    fun scan() {
        TacWebHost.call("codeScanner", "scan")
    }

    fun cancel() {
        TacWebHost.call("codeScanner", "cancel")
    }
}
