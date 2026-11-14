package dev.zaturo.bot

import net.schmizz.sshj.common.Buffer
import java.security.MessageDigest
import java.security.PublicKey
import java.util.Base64

/** Hash exactly the RFC4253/OpenSSH wire public key, never Java X509 encoding. */
object HostPin {
    fun matches(expected: String, publicKey: PublicKey): Boolean {
        if (!expected.startsWith("SHA256:")) return false
        return try {
            val wanted = Base64.getDecoder().decode(expected.substring(7))
            val wire = Buffer.PlainBuffer().putPublicKey(publicKey).compactData
            val observed = MessageDigest.getInstance("SHA-256").digest(wire)
            wanted.size == 32 && MessageDigest.isEqual(wanted, observed)
        } catch (_: Exception) { false }
    }
}
