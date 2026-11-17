package dev.zaturo.bot

import org.junit.Assert.*
import org.junit.Test

class PrivateCaptureCommandTest {
    @Test fun nativeRequestsKeepUrlsOutOfOrdinaryChat() {
        assertEquals(PrivateCaptureCommand.Request.Page("https://example.test/a?fixture=1"),PrivateCaptureCommand.parse("tire um print da pagina https://example.test/a?fixture=1"))
        assertEquals(PrivateCaptureCommand.Request.Messages,PrivateCaptureCommand.parse("envie print da tela de mensagens SMS do celular"))
        assertEquals(PrivateCaptureCommand.Request.Declined,PrivateCaptureCommand.parse("não tire print https://example.test"))
        assertEquals(PrivateCaptureCommand.Request.Choose,PrivateCaptureCommand.parse("print da pagina"))
        assertNull(PrivateCaptureCommand.parse("pesquise o clima"))
    }
    @Test fun currentPhonePrintNeverDependsOnAnSmsEvent() {
        for(text in listOf("enviar print do sm","mande o print da mensagem","print da tela atual do celular","envie print dos SMS mesmo que nenhum SMS foi enviado","mande print do SMS que não chegou","não envie SMS, mande um print do celular","o SMS não chegou, envie print do celular"))
            assertEquals(text,PrivateCaptureCommand.Request.Messages,PrivateCaptureCommand.parse(text))
        for(text in listOf("não tire um print do SMS","não envie print do celular","nunca capture a tela de mensagens","se chegar SMS mande print"))
            assertEquals(text,PrivateCaptureCommand.Request.Declined,PrivateCaptureCommand.parse(text))
        assertNull(PrivateCaptureCommand.parse("abrir sms"))
    }
    @Test fun urlIsBoundedAndCannotEmbedCredentialsOrLocalFileSchemes() {
        for(value in listOf("file:///etc/passwd","https://user:pass@example.test/","https://example.test/\nsecret","https://example.test/"+"a".repeat(2048))) assertFalse(PrivateCaptureCommand.validUrl(value))
        assertTrue(PrivateCaptureCommand.validUrl("https://example.test/página"))
    }
}
