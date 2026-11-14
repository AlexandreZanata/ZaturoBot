package dev.zaturo.bot

import android.app.Activity
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import kage.crypto.x25519.X25519Identity
import org.json.JSONObject
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/** Endpoint-only age identity, ciphertext transport and a finite RAM-only image inside Hub. */
internal class PrivatePhoneView(private val activity: Activity, private val ui: MobileUi,
                       private val api: () -> HubApi, private val attach: (View) -> Unit,
                       private val status: (String) -> Unit) {
    private val stored = PrivateSettings(activity,"private_viewer")
    private val executor = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private var cancelled = AtomicBoolean(true)
    private var session: String? = null
    private var image: ImageView? = null
    private var bitmap: Bitmap? = null
    private var card: View? = null
    private var revision = 0L
    private class PrivateFailure(val safeText: String): Exception()
    fun start(worker: String,device: String) {
        clear()
        val current=++revision
        val stop=AtomicBoolean(false);cancelled=stop
        status("Solicitando captura privada do celular no computador…")
        executor.execute {
            var created: String?=null
            try {
                val identity=stored.read()?.optString("identity")?.takeIf {it.isNotEmpty()}?.let {X25519Identity.decode(it)}
                    ?: X25519Identity.new().also {stored.save(JSONObject().put("identity",it.encodeToString()))}
                check(!stop.get())
                val client=api();client.registerViewer(identity.recipient().encodeToString())
                check(!stop.get())
                val initial=client.privateView(worker,device);created=initial.getString("id")
                val id=created
                main.post {if(current==revision && !stop.get()) session=id}
                val end=SystemClock.elapsedRealtime()+55000
                var result=initial
                var measuredAt=SystemClock.elapsedRealtime()
                while(result.getString("state")=="waiting" && SystemClock.elapsedRealtime()<end && !stop.get()) {
                    Thread.sleep(1000);measuredAt=SystemClock.elapsedRealtime();result=client.privateStatus(id)
                }
                check(!stop.get())
                if(result.getString("state")!="ready") throw PrivateFailure(when(result.optString("failure_code")) {
                    "android_disconnected"->"O celular registrado está desconectado do USB no PC de trabalho. Verifique o cabo e reconecte o aparelho."
                    "android_authorization_required"->"O celular exige autorização USB presencial. Desbloqueie o aparelho e aceite a chave RSA do PC de trabalho."
                    "android_busy"->"O celular está ocupado por outra sessão autorizada. Aguarde o encerramento dela."
                    "policy_denied"->"A captura não está autorizada no cadastro local do worker. O Codex do PC de trabalho deve conferir o aparelho e a permissão de visualização."
                    "android_unavailable"->"O ADB não conseguiu produzir uma captura válida dentro do limite. Confira o serviço local, a conexão USB e as proteções do aparelho."
                    else->"A captura não ficou pronta dentro do prazo ou foi cancelada. Nenhuma proteção será contornada; solicite uma nova captura quando o celular estiver pronto."
                })
                // The local monotonic limit starts before downloading, so transport cannot extend it.
                val remaining=(result.getLong("remaining_millis").coerceIn(0,60000)-(SystemClock.elapsedRealtime()-measuredAt)).coerceAtLeast(0)
                check(remaining>0)
                val deadline=SystemClock.elapsedRealtime()+remaining
                val cipher=client.privateFrame(id)
                val plain=try {PrivateImageCrypto.decrypt(identity,cipher)} finally {cipher.fill(0)}
                val decoded=try {decodePng(plain)} finally {plain.fill(0)}
                main.post {
                    if(current!=revision || stop.get() || SystemClock.elapsedRealtime()>=deadline || activity.isFinishing) {decoded.recycle();return@post}
                    bitmap=decoded
                    val layout=LinearLayout(activity).apply {orientation=LinearLayout.VERTICAL;setPadding(ui.dp(12),ui.dp(12),ui.dp(12),ui.dp(12));background=ui.rounded()}
                    layout.addView(ui.text("Tela privada · expira em até 60 segundos",12,MobileUi.YELLOW,true))
                    val picture=ImageView(activity).apply {setImageBitmap(decoded);adjustViewBounds=true;contentDescription="Captura privada temporária do celular";isSaveEnabled=false}
                    image=picture;layout.addView(picture,LinearLayout.LayoutParams(-1,-2))
                    layout.addView(ui.button("Fechar captura",R.drawable.ic_close){clear()})
                    card=layout;attach(layout);status("Captura privada recebida. A imagem não entra no histórico.")
                    main.postDelayed({if(current==revision) clear()},(deadline-SystemClock.elapsedRealtime()).coerceAtLeast(0))
                }
            } catch (error: Exception) {
                val explanation=when(error) {
                    is PrivateFailure->error.safeText
                    is HubFailure->when(error.status) {403->"Este aplicativo ou worker não tem a permissão de visualização privada. Verifique o cadastro no hub e no PC de trabalho.";404->"A captura privada expirou ou já foi consumida. Solicite uma nova imagem.";409,429->"Há outra captura ativa ou o limite de sessões foi atingido. Aguarde e tente uma nova solicitação.";else->"O hub não concluiu a captura privada. Verifique a conexão e a versão do serviço."}
                    is java.net.SocketTimeoutException->"A conexão com o hub excedeu o prazo. A captura não será repetida automaticamente."
                    else->"Não foi possível autenticar ou abrir a imagem privada. Nenhum conteúdo foi salvo; verifique a conexão e solicite uma nova captura."
                }
                main.post {if(current==revision && !stop.get()) status(explanation)}
            } finally {
                if(stop.get() || current!=revision) created?.let {runCatching {api().cancelPrivate(it)}}
            }
        }
    }
    fun cardView(): View? = card
    fun clear() {
        revision++;cancelled.set(true)
        image?.setImageDrawable(null);image=null
        bitmap?.recycle();bitmap=null
        (card?.parent as? android.view.ViewGroup)?.removeView(card);card=null
        val old=session;session=null
        if(old!=null) executor.execute {runCatching {api().cancelPrivate(old)}}
    }
    fun close() {clear();executor.shutdown()}
    companion object {
        fun decodePng(bytes: ByteArray): Bitmap {
            require(bytes.size<=8*1024*1024 && bytes.size>=24 && bytes.take(8).toByteArray().contentEquals(byteArrayOf(-119,80,78,71,13,10,26,10)))
            val size=BitmapFactory.Options().apply {inJustDecodeBounds=true}
            BitmapFactory.decodeByteArray(bytes,0,bytes.size,size)
            require(size.outWidth in 1..4096 && size.outHeight in 1..4096 && size.outWidth.toLong()*size.outHeight<=16*1024*1024)
            return requireNotNull(BitmapFactory.decodeByteArray(bytes,0,bytes.size))
        }
    }
}
