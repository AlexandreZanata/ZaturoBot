package dev.zaturo.bot

import android.app.Activity
import android.app.DatePickerDialog
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.*
import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Base64
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/** Native Kotlin owner companion. No SMS permission, arbitrary commands or background reader. */
class MainActivity : Activity() {
    private val background=Executors.newSingleThreadExecutor()
    private val main=Handler(Looper.getMainLooper())
    private val busy=AtomicBoolean(false)
    private lateinit var privateSettings: PrivateSettings
    private lateinit var tunnel: SshTunnel
    private lateinit var container: LinearLayout
    private lateinit var status: TextView
    private lateinit var feed: LinearLayout
    private lateinit var dayButton: Button
    private lateinit var search: EditText
    private lateinit var kind: Spinner
    private lateinit var draft: EditText
    private var visible=false
    private var currentPage="hub"
    private var selectedDay=LocalDate.now()
    private var configuration: JSONObject?=null
    private var enrollment: Enrollment?=null
    private var loadedOffset=0
    private var generation=0L
    private val feedItems=mutableListOf<JSONObject>()
    private class Pending(val message:String,val worker:String?,val action:String?,val id:String)
    private var pending: Pending?=null
    private val poll=object:Runnable {override fun run(){if(visible){if(currentPage=="hub"&&!busy.get())refresh(false);main.postDelayed(this,10000)}}}
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        privateSettings=PrivateSettings(this);configuration=privateSettings.read();enrollment=configuration?.let {parseEnrollmentOrNull(it)}
        tunnel=SshTunnel {_,message->main.post {if(visible){status.text=message;if(tunnel.ready)refresh(false)}}}
        buildShell();showHub()
    }
    override fun onStart(){super.onStart();visible=true;enrollment?.let {tunnel.start(it)};if(enrollment==null)refresh(false);main.postDelayed(poll,10000)}
    override fun onStop(){visible=false;main.removeCallbacks(poll);tunnel.stop();super.onStop()}
    override fun onDestroy(){background.shutdownNow();super.onDestroy()}
    private fun buildShell(){
        val outer=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL;setBackgroundColor(BG);setPadding(dp(18),dp(12),dp(18),dp(12))}
        outer.setOnApplyWindowInsetsListener {v,insets->v.setPadding(dp(18),insets.systemWindowInsetTop+dp(12),dp(18),insets.systemWindowInsetBottom+dp(12));insets}
        val brand=LinearLayout(this).apply {gravity=Gravity.CENTER_VERTICAL}
        brand.addView(ImageView(this).apply {setImageResource(R.drawable.zaturo_mark)},LinearLayout.LayoutParams(dp(44),dp(44)))
        brand.addView(text("ZaturoBot",25,WHITE,true),LinearLayout.LayoutParams(0,dp(54),1f))
        brand.addView(button("Ajustes"){showSettings()})
        outer.addView(brand)
        status=text("Conecte seu hub privado.",12,MUTED).apply {setPadding(0,dp(8),0,dp(14))};outer.addView(status)
        val navigation=LinearLayout(this)
        navigation.addView(button("Hub"){showHub()},LinearLayout.LayoutParams(0,dp(45),1f))
        navigation.addView(button("Serviços"){showServices()},LinearLayout.LayoutParams(0,dp(45),1f))
        navigation.addView(button("Reconectar"){enrollment?.let {tunnel.start(it)} ?: refresh(false)},LinearLayout.LayoutParams(0,dp(45),1f))
        outer.addView(navigation)
        container=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL;setPadding(0,dp(18),0,0)};outer.addView(container,LinearLayout.LayoutParams(-1,0,1f));setContentView(outer)
    }
    private fun showHub(){
        currentPage="hub";container.removeAllViews();container.addView(text("Seu dia, em movimento.",23,WHITE,true));container.addView(text("Histórico salvo. Ações autorizadas.",12,MUTED))
        val filters=LinearLayout(this).apply {setPadding(0,dp(14),0,dp(8));gravity=Gravity.CENTER_VERTICAL}
        dayButton=button(selectedDay.toString()){DatePickerDialog(this,{_,y,m,d->selectedDay=LocalDate.of(y,m+1,d);dayButton.text=selectedDay.toString();generation++;refresh(false)},selectedDay.year,selectedDay.monthValue-1,selectedDay.dayOfMonth).show()};filters.addView(dayButton,LinearLayout.LayoutParams(0,dp(44),1f))
        kind=Spinner(this).apply {adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,listOf("Tudo","Conversas","Tarefas","Arquivos","Android","YouTube"))};filters.addView(kind,LinearLayout.LayoutParams(0,dp(44),1f));container.addView(filters)
        val queryRow=LinearLayout(this)
        search=edit("Pesquisar no histórico");queryRow.addView(search,LinearLayout.LayoutParams(0,dp(46),1f));queryRow.addView(button("Filtrar"){generation++;refresh(false)});container.addView(queryRow)
        val scroll=ScrollView(this).apply {isFillViewport=true};feed=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL;setPadding(0,dp(14),0,dp(14))};scroll.addView(feed);container.addView(scroll,LinearLayout.LayoutParams(-1,0,1f));renderFeed(false)
        draft=edit("O que você quer acompanhar ou fazer?").apply {minLines=2;maxLines=4;inputType=InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE};container.addView(draft,LinearLayout.LayoutParams(-1,dp(88)))
        val sendRow=LinearLayout(this)
        sendRow.addView(button("Diagnóstico"){submit("Execute o diagnóstico autorizado do computador selecionado.","diagnostic")},LinearLayout.LayoutParams(0,dp(48),1f))
        sendRow.addView(button("Enviar →",true){submit(draft.text.toString(),null)},LinearLayout.LayoutParams(0,dp(48),1f));container.addView(sendRow)
        if(visible)refresh(false)
    }
    private fun renderFeed(hasMore:Boolean){
        feed.removeAllViews()
        if(feedItems.isEmpty())feed.addView(text(if(configuration==null)"Configure a conexão para abrir seu hub." else "Nenhuma atividade neste filtro.",15,MUTED).apply {setPadding(0,dp(22),0,dp(22))})
        for(item in feedItems){
            val card=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL;background=rounded(if(item.optString("role")=="owner")0xff242116.toInt() else SURFACE);setPadding(dp(16),dp(13),dp(16),dp(15))}
            val role=when(item.optString("role")){"owner","user"->"Você";"assistant"->"ZaturoBot";else->"Sistema"}
            val time=runCatching {Instant.ofEpochMilli(item.getLong("created_at")).atZone(ZoneId.systemDefault()).format(DateTimeFormatter.ofPattern("dd/MM · HH:mm"))}.getOrDefault("")
            card.addView(text("$role · $time",11,if(role=="Você")YELLOW else MUTED,true))
            card.addView(text(item.optString("text"),15,WHITE).apply {setPadding(0,dp(9),0,0)})
            if(!item.isNull("task_id"))card.addView(text("Tarefa · "+item.optString("task_id"),10,MUTED).apply {typeface=Typeface.MONOSPACE;setPadding(0,dp(8),0,0)})
            if(!item.isNull("artifact_id"))card.addView(text("Imagem disponível por entrega criptografada no computador. A prévia móvel de anexos cifrados ainda não está disponível nesta versão inicial.",11,YELLOW).apply {setPadding(0,dp(8),0,0)})
            feed.addView(card,LinearLayout.LayoutParams(-1,-2).apply {bottomMargin=dp(10)})
        }
        if(hasMore&&feedItems.size<200)feed.addView(button("Carregar mais"){refresh(true)})
    }
    private fun api():HubApi {
        val config=configuration ?: error("Connection is not configured")
        val origin=if(enrollment!=null){require(tunnel.ready);SshTunnel.ORIGIN}else config.getString("hub_url")
        return HubApi(origin,config.getString("app_token"))
    }
    private fun refresh(append:Boolean){
        if(!visible||currentPage!="hub"||configuration==null||!busy.compareAndSet(false,true))return
        val current=generation;val day=selectedDay;val query=search.text.toString();val selectedKind=listOf("","message","task","file","android","youtube")[kind.selectedItemPosition.coerceAtLeast(0)];val offset=if(append)loadedOffset else 0
        background.execute {
            try {val result=api().feed(day,selectedKind,query,offset);val items=result.getJSONArray("items");val safe=mutableListOf<JSONObject>();for(n in 0 until items.length()){val item=items.getJSONObject(n);if(!item.optBoolean("sensitive",false))safe.add(item)}
                main.post {if(visible&&currentPage=="hub"&&current==generation){if(!append)feedItems.clear();feedItems.addAll(safe);loadedOffset=offset+items.length();renderFeed(result.optBoolean("has_more"));status.text="Hub sincronizado · conexão privada"}}
            }catch(_:Exception){main.post {if(visible)status.text="Hub indisponível. Confira conexão e credencial; o histórico foi preservado."}}
            finally{busy.set(false)}
        }
    }
    private fun submit(message:String,action:String?){
        if(message.isBlank()||!busy.compareAndSet(false,true))return
        val worker=if(action!=null)configuration?.optString("target_worker","work-pc")else null
        val previous=pending
        val request=if(previous!=null&&previous.message==message&&previous.action==action&&previous.worker==worker)previous else Pending(message,worker,action,UUID.randomUUID().toString()).also {pending=it}
        status.text="Enviando solicitação…"
        background.execute {
            try {api().chat(request.message,request.worker,request.action,request.id)
                main.post {pending=null;if(currentPage=="hub"&&action==null&&draft.text.toString()==message)draft.setText("");status.text="Solicitação registrada no hub.";busy.set(false);if(currentPage!="hub")showHub()else refresh(false)}
            }catch(_:Exception){main.post {status.text="Não foi possível confirmar. Toque novamente para reconciliar a mesma solicitação."};busy.set(false)}
        }
    }
    private fun showServices(){
        currentPage="services";container.removeAllViews();container.addView(text("Serviços do seu bot",23,WHITE,true));container.addView(text("Executados no computador selecionado. Nenhuma disponibilidade é presumida.",13,MUTED).apply {setPadding(0,dp(10),0,dp(20))})
        container.addView(serviceCard("YouTube","Perfis e contas por canal. A geração, extensão e upload continuam no serviço existente.","Listar perfis"){submit("Liste os perfis do serviço YouTube.","youtube_profiles")})
        container.addView(serviceCard("Android por USB","Descoberta de dispositivos e estado da autorização. Códigos e telas sensíveis não entram neste histórico.","Listar dispositivos"){submit("Liste os dispositivos Android no computador.","android_devices")})
        container.addView(text("Arquivos são cifrados entre os computadores. Este aplicativo usa sua própria credencial de acesso ao hub.",13,MUTED).apply {setPadding(0,dp(22),0,0)})
    }
    private fun serviceCard(name:String,description:String,label:String,action:()->Unit):View {return LinearLayout(this).apply {orientation=LinearLayout.VERTICAL;background=rounded(SURFACE);setPadding(dp(18),dp(18),dp(18),dp(18));addView(text(name,19,WHITE,true));addView(text(description,13,MUTED).apply {setPadding(0,dp(8),0,dp(14))});addView(button(label){action()});layoutParams=LinearLayout.LayoutParams(-1,-2).apply {bottomMargin=dp(16)}}}
    private fun showSettings(){
        currentPage="settings";container.removeAllViews();val scroll=ScrollView(this);val form=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL};scroll.addView(form);container.addView(scroll)
        form.addView(text("Conecte seu espaço.",25,WHITE,true));form.addView(text("Importe o arquivo privado emitido pelo seu hub. Ele usa uma chave SSH exclusiva para este aplicativo.",14,MUTED).apply {setPadding(0,dp(12),0,dp(18))})
        form.addView(button("Importar conexão privada",true){startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply {addCategory(Intent.CATEGORY_OPENABLE);type="application/json"},IMPORT_CONNECTION)})
        form.addView(text(if(enrollment!=null)"Conexão SSH configurada. A chave permanece protegida no Android Keystore." else "Sem conexão SSH importada.",12,YELLOW).apply {setPadding(0,dp(12),0,dp(28))})
        form.addView(text("Conexão HTTPS ou túnel local existente",17,WHITE,true));val origin=edit("https://seu-hub ou http://127.0.0.1:17878");origin.setText(configuration?.optString("hub_url","")?:"");form.addView(origin)
        val token=edit("Token do aplicativo (deixe vazio para manter)").apply {inputType=InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD};form.addView(token)
        val worker=edit("Computador de execução");worker.setText(configuration?.optString("target_worker","work-pc")?:"work-pc");form.addView(worker)
        form.addView(button("Salvar conexão direta"){try{val value=JSONObject().put("hub_url",EndpointPolicy.validate(origin.text.toString())).put("app_token",EndpointPolicy.token(token.text.toString().ifEmpty {configuration?.optString("app_token","")?:""})).put("target_worker",EndpointPolicy.identifier(worker.text.toString()));privateSettings.save(value);configuration=value;enrollment=null;tunnel.stop();token.setText("");feedItems.clear();generation++;showHub()}catch(_:Exception){status.text="Use uma origem HTTPS ou loopback e uma credencial válida."}})
        form.addView(text("A configuração é cifrada com AES-GCM e chave do Android Keystore. Backup e capturas do aplicativo estão desabilitados. O túnel existe apenas enquanto o aplicativo está visível.",12,MUTED).apply {setPadding(0,dp(18),0,dp(20))})
        form.addView(button("Remover conexão deste aplicativo"){privateSettings.clear();configuration=null;enrollment=null;tunnel.stop();feedItems.clear();generation++;status.text="Conexão local removida. A revogação no hub é feita separadamente.";showSettings()})
    }
    @Deprecated("Activity result override for native platform Activity")
    override fun onActivityResult(requestCode:Int,resultCode:Int,data:Intent?){
        super.onActivityResult(requestCode,resultCode,data)
        if(requestCode!=IMPORT_CONNECTION||resultCode!=RESULT_OK)return
        try {val uri=data?.data ?: return;val bytes=contentResolver.openInputStream(uri)!!.use {input->val out=java.io.ByteArrayOutputStream();val buffer=ByteArray(4096);while(true){val count=input.read(buffer);if(count<0)break;require(out.size()+count<=32768);out.write(buffer,0,count)};out.toByteArray()};val value=JSONObject(String(bytes,Charsets.UTF_8));bytes.fill(0)
            require(value.getInt("schema")==1);val imported=parseEnrollmentOrNull(value)?:error("Invalid enrollment")
            privateSettings.save(value);configuration=value;enrollment=imported;feedItems.clear();generation++;status.text="Conexão importada com segurança.";showHub();if(visible)tunnel.start(imported)
        }catch(_:Exception){status.text="Arquivo de conexão inválido. Nenhuma credencial foi exibida ou substituída."}
    }
    private fun parseEnrollmentOrNull(value:JSONObject):Enrollment? {return try {if(value.optInt("schema")!=1)return null;val decoded=Base64.getDecoder().decode(value.getString("ssh_private_key_b64"));require(decoded.size<=16384);val key=String(decoded,Charsets.UTF_8);decoded.fill(0);Enrollment(value.getString("client_id"),value.getString("ssh_host"),value.getInt("ssh_port"),value.getString("ssh_user"),key,value.getString("host_key_sha256"),value.getString("app_token"),value.getString("target_worker"))}catch(_:Exception){null}}
    private fun text(value:String,size:Int,color:Int,bold:Boolean=false)=TextView(this).apply {text=value;textSize=size.toFloat();setTextColor(color);if(bold)setTypeface(typeface,Typeface.BOLD)}
    private fun button(value:String,primary:Boolean=false,action:()->Unit)=Button(this).apply {text=value;isAllCaps=false;textSize=12f;setTextColor(if(primary)BG else WHITE);background=rounded(if(primary)YELLOW else SURFACE);setPadding(dp(12),dp(5),dp(12),dp(5));minHeight=dp(42);setOnClickListener {action()}}
    private fun edit(hintText:String)=EditText(this).apply {hint=hintText;textSize=14f;setTextColor(WHITE);setHintTextColor(MUTED);setPadding(dp(12),dp(8),dp(12),dp(8));background=rounded(SURFACE);layoutParams=LinearLayout.LayoutParams(-1,-2).apply {bottomMargin=dp(12)}}
    private fun rounded(color:Int)=GradientDrawable().apply {setColor(color);cornerRadius=dp(10).toFloat()}
    private fun dp(value:Int)=(value*resources.displayMetrics.density).toInt()
    companion object {private const val IMPORT_CONNECTION=41;private const val BG=0xff0b0b0d.toInt();private const val SURFACE=0xff1a1a1e.toInt();private const val YELLOW=0xffe5b900.toInt();private const val WHITE=0xffffffff.toInt();private const val MUTED=0xff92919c.toInt()}
}
