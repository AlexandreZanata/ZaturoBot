package dev.zaturo.bot

/** No SMS/event/change count participates in one-shot delivery. */
internal object PreparedCapturePolicy {
    enum class Decision { WAIT, OPEN, STOP }
    fun decide(state: String, focusedHub: Boolean, remainingMillis: Long): Decision {
        if(!focusedHub || remainingMillis<=0) return Decision.STOP
        return when(state) {
            "prepared","waiting" -> Decision.WAIT
            "ready" -> Decision.OPEN
            else -> Decision.STOP
        }
    }
}
