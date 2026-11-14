package dev.zaturo.bot

import net.schmizz.sshj.SSHClient
import net.schmizz.sshj.transport.verification.HostKeyVerifier
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test
import java.net.HttpURLConnection
import java.net.URL
import java.nio.file.Files
import java.nio.file.Path
import java.security.PublicKey
import java.util.Base64

/** Explicit opt-in smoke: only an owner-issued restricted mobile key, no real SMS or accounts. */
class LiveSshOptInTest {
    @Test fun mobileKeyForwardsHubAndWrongHostPinFailsClosed() {
        val path=System.getenv("ZATURO_ANDROID_SSH_TEST_BUNDLE")
        assumeTrue("Owner must supply a private mobile enrollment path",path!=null)
        val bytes=Files.readAllBytes(Path.of(path!!));require(bytes.size<=32768)
        val bundle=JSONObject(String(bytes,Charsets.UTF_8));bytes.fill(0)
        val key=String(Base64.getDecoder().decode(bundle.getString("ssh_private_key_b64")),Charsets.UTF_8)
        val enrolled=Enrollment(bundle.getString("client_id"),bundle.getString("ssh_host"),bundle.getInt("ssh_port"),bundle.getString("ssh_user"),key,bundle.getString("host_key_sha256"),bundle.getString("app_token"),bundle.getString("target_worker"))
        val tunnel=SshTunnel {_,_->}
        try {
            tunnel.start(enrolled)
            val deadline=System.nanoTime()+30_000_000_000L
            while(!tunnel.ready&&System.nanoTime()<deadline)Thread.sleep(100)
            assertTrue("Pinned mobile transport did not become ready",tunnel.ready)
            val connection=URL(SshTunnel.ORIGIN+"/healthz").openConnection() as HttpURLConnection
            connection.connectTimeout=5000;connection.readTimeout=5000
            try {assertEquals(200,connection.responseCode);val health=JSONObject(connection.inputStream.use {String(it.readBytes(),Charsets.UTF_8)});assertEquals("ok",health.getString("status"))}finally {connection.disconnect()}
        }finally {tunnel.stop()}
        // A separate single handshake with a wrong pin must stop before authentication.
        SSHClient().use {ssh ->
            ssh.connectTimeout=10000;ssh.timeout=10000
            ssh.addHostKeyVerifier(object:HostKeyVerifier {
                override fun verify(hostname:String,port:Int,key:PublicKey):Boolean=HostPin.matches("SHA256:"+Base64.getEncoder().withoutPadding().encodeToString(ByteArray(32)),key)
                override fun findExistingAlgorithms(hostname:String,port:Int):List<String> = listOf("ssh-ed25519")
            })
            assertThrows(Exception::class.java){ssh.connect(enrolled.host,enrolled.port)}
            assertFalse(ssh.isAuthenticated)
        }
    }
}
