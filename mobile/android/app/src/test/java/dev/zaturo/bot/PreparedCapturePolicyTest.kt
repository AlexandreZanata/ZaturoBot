package dev.zaturo.bot

import org.junit.Assert.*
import org.junit.Test

class PreparedCapturePolicyTest {
    @Test fun readyImageOpensRegardlessOfSmsOrPixelChange() {
        assertEquals(PreparedCapturePolicy.Decision.WAIT,PreparedCapturePolicy.decide("prepared",true,45000))
        assertEquals(PreparedCapturePolicy.Decision.WAIT,PreparedCapturePolicy.decide("waiting",true,44000))
        // A one-shot frame is ready even when the inbox and its pixels have not changed.
        repeat(2) {assertEquals(PreparedCapturePolicy.Decision.OPEN,PreparedCapturePolicy.decide("ready",true,43000))}
        assertEquals(PreparedCapturePolicy.Decision.STOP,PreparedCapturePolicy.decide("unchanged",true,43000))
    }
    @Test fun noImageIsOpenedOutsideFocusedHubOrAfterExpiry() {
        assertEquals(PreparedCapturePolicy.Decision.STOP,PreparedCapturePolicy.decide("ready",false,43000))
        assertEquals(PreparedCapturePolicy.Decision.STOP,PreparedCapturePolicy.decide("ready",true,0))
        for(state in listOf("failed","cancelled","consumed","invalid"))
            assertEquals(PreparedCapturePolicy.Decision.STOP,PreparedCapturePolicy.decide(state,true,43000))
    }
}
