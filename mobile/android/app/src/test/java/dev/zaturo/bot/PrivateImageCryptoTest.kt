package dev.zaturo.bot

import kage.Age
import kage.crypto.x25519.X25519Identity
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test

class PrivateImageCryptoTest {
    private fun encrypt(identity: X25519Identity,bytes: ByteArray): ByteArray {
        val output=ByteArrayOutputStream();Age.encryptStream(listOf(identity.recipient()),ByteArrayInputStream(bytes),output);return output.toByteArray()
    }
    @Test fun receiverRejectsWrongRecipientHeaderAndPayloadTampering() {
        val identity=X25519Identity.new();val plain="Synthetic private image bytes; no real phone".toByteArray()
        val cipher=encrypt(identity,plain)
        assertArrayEquals(plain,PrivateImageCrypto.decrypt(identity,cipher))
        assertTrue(runCatching {PrivateImageCrypto.decrypt(X25519Identity.new(),cipher)}.isFailure)
        for(index in listOf(0,cipher.lastIndex)) {val bad=cipher.copyOf();bad[index]=(bad[index].toInt() xor 1).toByte();assertTrue(runCatching {PrivateImageCrypto.decrypt(identity,bad)}.isFailure)}
        assertTrue(runCatching {PrivateImageCrypto.decrypt(identity,cipher.copyOf(cipher.size-1))}.isFailure)
        assertTrue(runCatching {PrivateImageCrypto.decrypt(identity,ByteArray(PrivateImageCrypto.MAX_BYTES+65537))}.isFailure)
        plain.fill(0);cipher.fill(0)
    }
    @Test fun referenceAgeEncryptsToTheNativeAndroidIdentityAcrossStreamChunks() {
        val binary=listOf(java.io.File("/usr/bin/age"),java.io.File(System.getProperty("user.home"),".local/bin/age")).firstOrNull {it.canExecute()}
        assumeTrue("Reference age CLI is an optional interoperability fixture",binary!=null)
        val identity=X25519Identity.new();val plain=ByteArray(65536+123){(it%251).toByte()}
        val process=ProcessBuilder(requireNotNull(binary).path,"-r",identity.recipient().encodeToString()).redirectError(java.io.File("/dev/null")).start()
        val writer=Thread {process.outputStream.use {it.write(plain)}}.apply {start()}
        val cipher=process.inputStream.use {it.readBytes()};writer.join(5000);assertTrue(process.waitFor(5,java.util.concurrent.TimeUnit.SECONDS));assertEquals(0,process.exitValue())
        assertArrayEquals(plain,PrivateImageCrypto.decrypt(identity,cipher));plain.fill(0);cipher.fill(0)
    }
}
