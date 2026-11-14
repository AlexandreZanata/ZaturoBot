package dev.zaturo.bot

import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.time.LocalDate
import java.time.OffsetDateTime

class HubApi(origin: String, private val token: String) {
    private val base=EndpointPolicy.validate(origin)
    init { EndpointPolicy.token(token) }
    private fun request(path: String,body: JSONObject?=null): JSONObject {
        val connection=URL(base+path).openConnection(java.net.Proxy.NO_PROXY) as HttpURLConnection
        connection.useCaches=false;connection.instanceFollowRedirects=false;connection.connectTimeout=5000;connection.readTimeout=10000
        connection.setRequestProperty("Authorization","Bearer $token")
        try {
            if(body!=null){val data=body.toString().toByteArray(Charsets.UTF_8);require(data.size<=16384);connection.requestMethod="POST";connection.doOutput=true;connection.setRequestProperty("Content-Type","application/json");connection.setFixedLengthStreamingMode(data.size);connection.outputStream.use {it.write(data)}}
            require(connection.responseCode in 200..299) {"Hub returned HTTP ${connection.responseCode}"}
            val bytes=connection.inputStream.use {input ->val out=java.io.ByteArrayOutputStream();val buffer=ByteArray(8192);while(true){val count=input.read(buffer);if(count<0)break;require(out.size()+count<=1024*1024);out.write(buffer,0,count)};out.toByteArray()}
            return JSONObject(String(bytes,Charsets.UTF_8))
        } finally {connection.disconnect()}
    }
    fun feed(day: LocalDate,kind: String,query: String,offset: Int=0): JSONObject {
        require(query.length<=256 && offset in 0..1000000)
        val params="day=$day&offset_minutes=${OffsetDateTime.now().offset.totalSeconds/60}&limit=20&offset=$offset"+
            (if(kind.isEmpty())"" else "&kind="+URLEncoder.encode(kind,"UTF-8"))+(if(query.isBlank())"" else "&q="+URLEncoder.encode(query,"UTF-8"))
        val result=request("/v1/feed?$params");require(result.getJSONArray("items").length()<=20);return result
    }
    fun chat(message: String,worker: String?,action: String?,requestId: String): JSONObject {
        require(message.isNotBlank() && message.toByteArray(Charsets.UTF_8).size<=4096)
        val body=JSONObject().put("message",message).put("idempotency_key",requestId)
        worker?.let {body.put("target_worker",EndpointPolicy.identifier(it))};action?.let {require(it in setOf("diagnostic","youtube_profiles","android_devices"));body.put("action",it)}
        return request("/v1/chat",body)
    }
}
