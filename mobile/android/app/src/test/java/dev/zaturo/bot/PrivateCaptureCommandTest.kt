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
    @Test fun urlIsBoundedAndCannotEmbedCredentialsOrLocalFileSchemes() {
        for(value in listOf("file:///etc/passwd","https://user:pass@example.test/","https://example.test/\nsecret","https://example.test/"+"a".repeat(2048))) assertFalse(PrivateCaptureCommand.validUrl(value))
        assertTrue(PrivateCaptureCommand.validUrl("https://example.test/página"))
    }
}
