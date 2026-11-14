class Crypto {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("crypto", "supported"))

    fun check(): String = request("check", listOf())

    fun makeKeyPair(key: String, algorithm: String, keep: Boolean): String =
        request("makeKeyPair", listOf(key, algorithm, keep))

    fun publicKey(key: String): String = request("publicKey", listOf(key))

    fun sign(key: String, data: String): String = request("sign", listOf(key, data))

    fun verify(publicKey: String, data: String, signature: String): String =
        request("verify", listOf(publicKey, data, signature))

    fun agree(key: String, peerPublicKey: String, salt: String, info: String, newKey: String, use: String, keep: Boolean): String =
        request("agree", listOf(key, peerPublicKey, salt, info, newKey, use, keep))

    fun deriveKey(fromKey: String, salt: String, info: String, newKey: String, use: String, keep: Boolean): String =
        request("deriveKey", listOf(fromKey, salt, info, newKey, use, keep))

    fun importKey(newKey: String, secret: String, use: String, keep: Boolean): String =
        request("importKey", listOf(newKey, secret, use, keep))

    fun fromCode(newKey: String, code: String, salt: String, use: String, keep: Boolean): String =
        request("fromCode", listOf(newKey, code, salt, use, keep))

    fun encrypt(key: String, data: String, aad: String): String = request("encrypt", listOf(key, data, aad))

    fun decrypt(key: String, sealed: String, aad: String): String = request("decrypt", listOf(key, sealed, aad))

    fun hkdf(input: String, salt: String, info: String, length: Int): String =
        request("hkdf", listOf(input, salt, info, length))

    fun hash(data: String): String = request("hash", listOf(data))

    fun forget(key: String): String = request("forget", listOf(key))

    fun forgetAll(): String = request("forgetAll", listOf())

    private fun request(call: String, values: List<Any>): String = tacString(TacWebHost.call("crypto", call, values))
}
