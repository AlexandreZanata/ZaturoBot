package dev.zaturo.bot

import java.net.URI

/** Shared by tests and the native client; credentials never occur in URL components. */
object EndpointPolicy {
    fun validate(origin: String): String {
        require(origin.length <= 2048)
        val uri = URI(origin)
        require(uri.rawUserInfo == null && uri.rawQuery == null && uri.rawFragment == null)
        require(uri.path.isNullOrEmpty() || uri.path == "/")
        require(!uri.host.isNullOrBlank())
        require(uri.scheme == "https" || (uri.scheme == "http" && uri.host in setOf("127.0.0.1", "[::1]", "::1")))
        return origin.trimEnd('/')
    }
    fun token(value: String): String {
        require(value.length in 32..256 && value.all { it.code in 33..126 })
        return value
    }
    fun identifier(value: String): String {
        require(value.length in 1..64 && value.all { it.isLetterOrDigit() && it.code < 128 || it in "._-" })
        return value
    }
}
