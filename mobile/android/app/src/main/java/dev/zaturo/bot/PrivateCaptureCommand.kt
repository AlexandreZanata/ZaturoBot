package dev.zaturo.bot

import java.net.URI
import java.text.Normalizer

/** Native capture commands never enter ordinary chat/model history. */
internal object PrivateCaptureCommand {
    sealed class Request {
        data class Page(val url: String): Request()
        data object Messages: Request()
        data object Choose: Request()
        data object Declined: Request()
    }
    fun validUrl(value: String): Boolean = runCatching {
        val uri=URI(value)
        value.toByteArray(Charsets.UTF_8).size<=2048 && value.none {it.isWhitespace() || it.isISOControl() || it=='\\'} &&
            uri.scheme in setOf("http","https") && !uri.rawAuthority.isNullOrEmpty() && uri.rawUserInfo==null &&
            !uri.rawAuthority.contains('@') && !uri.rawAuthority.contains('%')
    }.getOrDefault(false)
    fun parse(message: String): Request? {
        val plain=Normalizer.normalize(message.trim().lowercase(),Normalizer.Form.NFD).replace(Regex("\\p{M}+"),"")
        if(!Regex("\\b(prints?|captur(a|e|ar)|screenshot)\\b").containsMatchIn(plain)) return null
        val denied=Regex("\\b(nao|nunca|sem)\\s+(?:(?:me|o|a|um|uma)\\s+)*(?:(?:envie|enviar|mande|mandar|tire|tirar|faca|fazer)\\s+(?:(?:me|o|a|um|uma)\\s+)*)?(?:prints?|captur(a|e|ar)|screenshot)\\b")
        if(denied.containsMatchIn(plain) || Regex("^(se|talvez|poderia)\\b").containsMatchIn(plain)) return Request.Declined
        val urls=Regex("https?://[^\\s]+",RegexOption.IGNORE_CASE).findAll(message).toList()
        if(urls.size==1 && validUrl(urls[0].value)) return Request.Page(urls[0].value)
        if(urls.isEmpty() && Regex("\\b(sms|sm|mensagem|mensagens|celular|telefone|smartphone)\\b").containsMatchIn(plain)) return Request.Messages
        return Request.Choose
    }
}
