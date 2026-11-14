package dev.zaturo.bot

import kage.Age
import kage.crypto.x25519.X25519Identity
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream

/** Standard age decoding only. No custom cipher, files, logs or model input. */
internal object PrivateImageCrypto {
    const val MAX_BYTES=8*1024*1024
    private class BoundedOutput : ByteArrayOutputStream() {
        override fun write(value: Int) {require(count<MAX_BYTES);super.write(value)}
        override fun write(bytes: ByteArray,offset: Int,length: Int) {require(length>=0 && count.toLong()+length<=MAX_BYTES);super.write(bytes,offset,length)}
        fun wipe() {buf.fill(0);reset()}
    }
    fun decrypt(identity: X25519Identity,cipher: ByteArray): ByteArray {
        require(cipher.size in 64..MAX_BYTES+65536)
        require(cipher.take(22).toByteArray().contentEquals("age-encryption.org/v1\n".toByteArray()))
        val output=BoundedOutput()
        return try {Age.decryptStream(listOf(identity),ByteArrayInputStream(cipher),output);output.toByteArray()} finally {output.wipe()}
    }
}
