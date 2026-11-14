package dev.zaturo.bot

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import org.json.JSONObject
import java.security.KeyStore
import java.util.Base64
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Tokens/SSH enrollment are stored as one AES-GCM blob protected by Android Keystore. */
class PrivateSettings(context: Context) {
    private val prefs = context.getSharedPreferences("private_connection", Context.MODE_PRIVATE)
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply {load(null)}
        val existing = store.getKey(ALIAS, null)
        if (existing is SecretKey) return existing
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(ALIAS,KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    fun save(value: JSONObject) {
        val bytes = value.toString().toByteArray(Charsets.UTF_8); require(bytes.size <= 32768)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding"); cipher.init(Cipher.ENCRYPT_MODE,key())
        val encrypted = cipher.doFinal(bytes)
        check(prefs.edit().putString("iv",Base64.getEncoder().encodeToString(cipher.iv)).putString("payload",Base64.getEncoder().encodeToString(encrypted)).commit())
        bytes.fill(0)
    }
    fun read(): JSONObject? {
        val payload = prefs.getString("payload",null) ?: return null
        return try {
            val cipher=Cipher.getInstance("AES/GCM/NoPadding");cipher.init(Cipher.DECRYPT_MODE,key(),GCMParameterSpec(128,Base64.getDecoder().decode(prefs.getString("iv", ""))))
            val bytes=cipher.doFinal(Base64.getDecoder().decode(payload));val value=JSONObject(String(bytes,Charsets.UTF_8));bytes.fill(0);value
        } catch (_: Exception) {null}
    }
    fun clear() { prefs.edit().clear().commit() }
    companion object {private const val ALIAS="zaturo-connection-v1"}
}
