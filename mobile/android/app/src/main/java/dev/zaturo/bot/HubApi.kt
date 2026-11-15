package dev.zaturo.bot

import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.time.LocalDate
import java.time.OffsetDateTime

class HubFailure(val status: Int): Exception()

class HubApi(origin: String, private val token: String) {
    private val base=EndpointPolicy.validate(origin)
    init { EndpointPolicy.token(token) }
    private fun bytes(path: String,body: JSONObject?=null,method: String?=null,maximum: Int=1024*1024): ByteArray {
        val connection=URL(base+path).openConnection(java.net.Proxy.NO_PROXY) as HttpURLConnection
        connection.useCaches=false;connection.instanceFollowRedirects=false;connection.connectTimeout=5000;connection.readTimeout=10000
        connection.setRequestProperty("Authorization","Bearer $token")
        connection.setRequestProperty("Cache-Control","no-store")
        method?.let {connection.requestMethod=it}
        try {
            if(body!=null){val data=body.toString().toByteArray(Charsets.UTF_8);require(data.size<=16384);connection.requestMethod=method ?: "POST";connection.doOutput=true;connection.setRequestProperty("Content-Type","application/json");connection.setFixedLengthStreamingMode(data.size);connection.outputStream.use {it.write(data)}}
            if(connection.responseCode !in 200..299) throw HubFailure(connection.responseCode)
            val bytes=connection.inputStream.use {input ->val out=java.io.ByteArrayOutputStream();val buffer=ByteArray(8192);while(true){val count=input.read(buffer);if(count<0)break;require(out.size()+count<=maximum);out.write(buffer,0,count)};out.toByteArray()}
            return bytes
        } finally {connection.disconnect()}
    }
    private fun request(path: String,body: JSONObject?=null): JSONObject = JSONObject(String(bytes(path,body),Charsets.UTF_8))
    fun sharedList(worker: String): JSONArray = JSONArray(String(bytes("/v1/shared/tasks?limit=10&target_worker="+URLEncoder.encode(EndpointPolicy.identifier(worker),"UTF-8")),Charsets.UTF_8))
    fun sharedRead(id: String): JSONObject = request("/v1/shared/tasks/"+EndpointPolicy.identifier(id))
    fun sharedUpdate(id: String,revision: Int,state: String,note: String,key: String): JSONObject = request("/v1/shared/tasks/"+EndpointPolicy.identifier(id)+"/updates",JSONObject().put("expected_revision",revision).put("state",state).put("note",note).put("idempotency_key",key))
    fun sharedCreate(worker: String,title: String,instructions: String,key: String): JSONObject = request("/v1/shared/tasks",JSONObject().put("target_worker",EndpointPolicy.identifier(worker)).put("title",title).put("instructions",instructions).put("idempotency_key",key))
    fun registerViewer(recipient: String): JSONObject = request("/v1/private-viewers",JSONObject().put("recipient",recipient).put("explicit_owner_confirmation",true))
    fun privateView(worker: String,device: String,source: String="android",prepared: Boolean=false,url: String?=null,returnToHub: Boolean=false): JSONObject {
        val body=JSONObject().put("source",source).put("worker_id",EndpointPolicy.identifier(worker)).put("device_id",EndpointPolicy.identifier(device)).put("watch_seconds",0).put("explicit_owner_confirmation",true)
        if(prepared) body.put("prepared",true)
        if(source=="browser") {require(url!=null && PrivateCaptureCommand.validUrl(url));body.put("url",url)} else require(url==null)
        if(source=="android_messages") body.put("return_to_hub",returnToHub) else require(!returnToHub)
        return request("/v1/private-views",body)
    }
    fun privateStatus(id: String): JSONObject = request("/v1/private-views/"+EndpointPolicy.identifier(id))
    fun privateFrame(id: String): ByteArray = bytes("/v1/private-views/"+EndpointPolicy.identifier(id)+"/content",maximum=8*1024*1024+65536)
    fun privateDisplayed(id: String) {bytes("/v1/private-views/"+EndpointPolicy.identifier(id)+"/displayed",method="POST",body=JSONObject().put("displayed",true),maximum=1024)}
    fun cancelPrivate(id: String) {bytes("/v1/private-views/"+EndpointPolicy.identifier(id),method="DELETE",maximum=1024)}
    fun feed(day: LocalDate,kind: String,query: String,offset: Int=0): JSONObject {
        require(query.length<=256 && offset in 0..1000000)
        val params="day=$day&offset_minutes=${OffsetDateTime.now().offset.totalSeconds/60}&limit=20&offset=$offset"+
            (if(kind.isEmpty())"" else "&kind="+URLEncoder.encode(kind,"UTF-8"))+(if(query.isBlank())"" else "&q="+URLEncoder.encode(query,"UTF-8"))
        val result=request("/v1/feed?$params");require(result.getJSONArray("items").length()<=20);return result
    }
    fun chat(message: String,worker: String?,action: String?,requestId: String): JSONObject {
        require(message.isNotBlank() && message.toByteArray(Charsets.UTF_8).size<=4096)
        val body=JSONObject().put("message",message).put("idempotency_key",requestId).put("action",action ?: if(message.trim().startsWith("/aprovar ") || message.trim().startsWith("/recusar ")) "computer_confirm" else "assistant")
        worker?.let {body.put("target_worker",EndpointPolicy.identifier(it))};action?.let {require(it in setOf("diagnostic","youtube_profiles","android_devices","android_open_messages"));body.put("action",it)}
        return request("/v1/chat",body)
    }
}
