package dev.zaturo.bot

import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.common.SecurityUtils
import org.bouncycastle.jce.provider.BouncyCastleProvider
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.security.Provider
import java.security.Security
import java.util.concurrent.atomic.AtomicLong

/** Lifecycle-bound, pinned, public-key-only local forward. No shell/session/remote forward APIs. */
class SshTunnel(private val status: (Boolean, String) -> Unit) {
    private val epoch = AtomicLong(0)
    @Volatile private var client: SSHClient? = null
    @Volatile private var socket: ServerSocket? = null
    @Volatile var ready = false; private set
    private val lock = Any()
    fun start(enrollment: Enrollment) {
        stop()
        val version = epoch.incrementAndGet()
        Thread({
            ensureCrypto()
            for (attempt in 0..3) {
                if (epoch.get() != version) break
                var ssh: SSHClient? = null
                var local: ServerSocket? = null
                try {
                    status(false, "Conectando túnel privado…")
                    ssh = SSHClient(); ssh.connectTimeout = 10000; ssh.timeout = 15000
                    val current = ssh
                    current.addHostKeyVerifier(object : net.schmizz.sshj.transport.verification.HostKeyVerifier {
                        override fun verify(hostname: String, port: Int, key: java.security.PublicKey): Boolean = hostname == enrollment.host && port == enrollment.port && HostPin.matches(enrollment.hostPin, key)
                        override fun findExistingAlgorithms(hostname: String, port: Int): List<String> = listOf("ssh-ed25519")
                    })
                    synchronized(lock) { if (epoch.get() != version) return@Thread; client = current }
                    current.connect(enrollment.host, enrollment.port)
                    current.authPublickey(enrollment.user, current.loadKeys(enrollment.key, null, null))
                    current.connection.keepAlive.keepAliveInterval = 15
                    local = ServerSocket(); local.reuseAddress = true; local.bind(InetSocketAddress("127.0.0.1", LOCAL_PORT), 8)
                    val server = local
                    synchronized(lock) { if (epoch.get() != version) return@Thread; socket = server; ready = true }
                    val parameters = net.schmizz.sshj.connection.channel.direct.Parameters("127.0.0.1", LOCAL_PORT, "127.0.0.1", 7878)
                    val forwarding = Thread({ try { current.newLocalPortForwarder(parameters, server).listen() } catch (_: Exception) {} }, "zaturo-forward").apply { isDaemon = true; start() }
                    status(true, "Túnel privado conectado")
                    while (epoch.get() == version && current.isConnected && current.isAuthenticated && forwarding.isAlive) Thread.sleep(500)
                } catch (_: Exception) {
                    if (epoch.get() == version) status(false, "Túnel indisponível. Confira rede, chave e identificação do servidor.")
                } finally {
                    try { local?.close() } catch (_: Exception) {}
                    try { ssh?.close() } catch (_: Exception) {}
                    synchronized(lock) { if (epoch.get() == version) {ready = false; client = null; socket = null} }
                }
                if (epoch.get() != version) break
                try { Thread.sleep((1000L shl attempt).coerceAtMost(8000)) } catch (_: InterruptedException) { break }
            }
            if (epoch.get() == version) status(false, "Túnel pausado após quatro tentativas. Toque em Reconectar.")
        }, "zaturo-ssh").apply {isDaemon=true;start()}
    }
    fun stop() {
        epoch.incrementAndGet(); ready = false
        val old = synchronized(lock) { val resources = Pair(client, socket); client = null; socket = null; resources }
        try { old.second?.close() } catch (_: Exception) {}
        Thread({try {old.first?.close()} catch (_: Exception) {}}, "zaturo-ssh-close").apply {isDaemon=true;start()}
    }
    companion object {
        const val LOCAL_PORT = 17879
        const val ORIGIN = "http://127.0.0.1:17879"
        @Synchronized private fun ensureCrypto() {
            // Register a separate alias: Android's built-in BC remains untouched for platform services.
            if (Security.getProvider("ZaturoSshBC") == null) {
                val dedicated = object : Provider("ZaturoSshBC", 1.87, "Zaturo SSH-only cryptography") {}
                dedicated.putAll(BouncyCastleProvider()); Security.addProvider(dedicated)
            }
            SecurityUtils.setRegisterBouncyCastle(false)
            SecurityUtils.setSecurityProvider("ZaturoSshBC")
        }
    }
}
