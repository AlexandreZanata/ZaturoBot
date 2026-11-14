package dev.zaturo.bot

/** No data-class toString(): this value contains an endpoint-only private SSH key. */
class Enrollment(val clientId: String, val host: String, val port: Int, val user: String,
                 val key: String, val hostPin: String, val appToken: String, val worker: String) {
    init {
        EndpointPolicy.identifier(clientId)
        require(host.matches(Regex("[A-Za-z0-9.-]{1,253}")) && port == 22 && user == "zaturo-link")
        require(key.length in 100..16384 && key.startsWith("-----BEGIN "))
        require(hostPin.matches(Regex("SHA256:[A-Za-z0-9+/]{43}=?")))
        EndpointPolicy.token(appToken); EndpointPolicy.identifier(worker)
    }
    override fun toString(): String = "Enrollment([REDACTED])"
}
