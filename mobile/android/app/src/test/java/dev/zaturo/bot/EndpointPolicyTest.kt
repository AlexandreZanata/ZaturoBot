package dev.zaturo.bot
import org.junit.Assert.*
import org.junit.Test
class EndpointPolicyTest {
    @Test fun secureOriginsExcludePublicPlaintextAndCredentialUrls(){
        assertEquals("http://127.0.0.1:17879",EndpointPolicy.validate("http://127.0.0.1:17879/"))
        assertEquals("https://hub.example",EndpointPolicy.validate("https://hub.example"))
        for(value in listOf("http://hub.example","http://127.0.0.1.evil.example","https://user:token@hub.example","https://hub.example/path","https://hub.example?q=token"))assertThrows(Exception::class.java){EndpointPolicy.validate(value)}
    }
    @Test fun credentialsAndWorkerIdentityFailClosed(){
        assertEquals("work-pc",EndpointPolicy.identifier("work-pc"))
        assertThrows(Exception::class.java){EndpointPolicy.identifier("../worker")}
        assertThrows(Exception::class.java){EndpointPolicy.token("too-short")}
        assertThrows(Exception::class.java){EndpointPolicy.token("a long value with whitespace cannot be a token")}
    }
}
