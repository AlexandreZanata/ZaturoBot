package dev.zaturo.bot
import net.schmizz.sshj.common.Buffer
import org.junit.Assert.*
import org.junit.Test
import java.security.KeyPairGenerator
import java.security.MessageDigest
import java.util.Base64
class HostPinTest {
    @Test fun pinsHashSshWireEncodingAndRejectAnotherKey(){
        val generator=KeyPairGenerator.getInstance("RSA").apply {initialize(2048)}
        val key=generator.generateKeyPair().public
        val wire=Buffer.PlainBuffer().putPublicKey(key).compactData
        val pin="SHA256:"+Base64.getEncoder().withoutPadding().encodeToString(MessageDigest.getInstance("SHA-256").digest(wire))
        assertTrue(HostPin.matches(pin,key));assertFalse(HostPin.matches(pin,generator.generateKeyPair().public))
        val wrongEncoding="SHA256:"+Base64.getEncoder().withoutPadding().encodeToString(MessageDigest.getInstance("SHA-256").digest(key.encoded))
        assertFalse(HostPin.matches(wrongEncoding,key));assertFalse(HostPin.matches("MD5:invalid",key));assertFalse(HostPin.matches("SHA256:malformed",key))
    }
}
