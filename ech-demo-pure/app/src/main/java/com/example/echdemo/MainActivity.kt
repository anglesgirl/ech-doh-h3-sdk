package com.example.echdemo

import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.Dns
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.URL
import java.net.UnknownHostException
import java.util.concurrent.TimeUnit

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val etDomain = findViewById<EditText>(R.id.etDomain)
        val etDoh = findViewById<EditText>(R.id.etDohServer)
        val etIp = findViewById<EditText>(R.id.etCustomIp)
        val btnTest = findViewById<Button>(R.id.btnTest)
        val tvResult = findViewById<TextView>(R.id.tvResult)

        btnTest.setOnClickListener {
            val domain = etDomain.text.toString().trim()
            if (domain.isEmpty()) {
                Toast.makeText(this, "请输入域名", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            val doh = etDoh.text.toString().trim().ifEmpty { "https://1.1.1.1/dns-query" }
            val ip = etIp.text.toString().trim()
            tvResult.text = "测试中...\n域名: $domain\nDoH: $doh\n指定IP: ${if (ip.isEmpty()) "(系统解析)" else ip}"
            btnTest.isEnabled = false
            CoroutineScope(Dispatchers.IO).launch {
                val out = runTest(domain, doh, ip)
                withContext(Dispatchers.Main) {
                    tvResult.text = out
                    btnTest.isEnabled = true
                }
            }
        }
    }

    private fun dohGet(dohServer: String, domain: String, qtype: String): JSONObject? {
        val c = (URL("$dohServer?name=$domain&type=$qtype").openConnection() as HttpURLConnection).apply {
            connectTimeout = 15000
            readTimeout = 15000
            requestMethod = "GET"
            setRequestProperty("Accept", "application/dns-json")
            setRequestProperty("User-Agent", "EchDemo/1.0")
        }
        return try {
            if (c.responseCode != 200) return null
            JSONObject(c.inputStream.bufferedReader().readText())
        } catch (e: Exception) {
            null
        } finally {
            c.disconnect()
        }
    }

    // 实测的地址来源：指定IP > 你的DoH A记录，绝不走系统解析
    private inner class DohDns(
        private val dohServer: String,
        private val fixedIp: String,
        private val target: String,
    ) : Dns {
        override fun lookup(hostname: String): List<InetAddress> {
            if (fixedIp.isNotEmpty() && hostname == target) {
                return listOf(InetAddress.getByName(fixedIp))
            }
            val json = dohGet(dohServer, hostname, "A")
                ?: throw UnknownHostException("DoH无响应: $hostname")
            val answers = json.optJSONArray("Answer")
                ?: throw UnknownHostException("DoH无A记录: $hostname")
            val out = mutableListOf<InetAddress>()
            for (i in 0 until answers.length()) {
                val o = answers.getJSONObject(i)
                if (o.optInt("type", 0) != 1) continue
                try {
                    out.add(InetAddress.getByName(o.optString("data", "")))
                } catch (_: Exception) {
                }
            }
            if (out.isEmpty()) throw UnknownHostException("DoH无有效IP: $hostname")
            return out
        }
    }

    private fun runTest(domain: String, dohServer: String, customIp: String): String {
        val sb = StringBuilder()
        sb.append("===== 测试结果 =====\n域名: $domain\nDoH: $dohServer\n指定IP: ${if (customIp.isEmpty()) "(无)" else customIp}\n\n")

        // 1. HTTPS 记录（ECH + H3 广告）
        sb.append("--- DoH HTTPS 记录 ---\n")
        try {
            val json = dohGet(dohServer, domain, "HTTPS")
            if (json == null) {
                sb.append("DoH查询失败\n")
            } else {
                val answers = json.optJSONArray("Answer")
                if (answers == null) {
                    sb.append("公网无HTTPS记录（Status=${json.optInt("Status")}）\n")
                    sb.append("说明：你本地注入的不影响公网，这项为空正常\n")
                } else {
                    var hasH3 = false
                    var hasEch = false
                    for (i in 0 until answers.length()) {
                        val data = answers.getJSONObject(i).optString("data", "")
                        sb.append("记录$i: $data\n")
                        if (data.contains("h3")) hasH3 = true
                        if (data.contains("ech=")) hasEch = true
                    }
                    sb.append("H3广告: ${if (hasH3) "✅" else "❌"}\n")
                    sb.append("ECH: ${if (hasEch) "✅ 含ech=" else "❌ 无ech="}\n")
                }
            }
        } catch (e: Exception) {
            sb.append("DoH异常: ${e.message}\n")
        }

        // 2. A 记录供参考
        sb.append("\n--- DoH A 记录 ---\n")
        try {
            val json = dohGet(dohServer, domain, "A")
            val answers = json?.optJSONArray("Answer")
            if (answers == null) sb.append("无A记录\n")
            else for (i in 0 until answers.length()) {
                sb.append("IP$i: ${answers.getJSONObject(i).optString("data", "")}\n")
            }
        } catch (e: Exception) {
            sb.append("A记录异常: ${e.message}\n")
        }

        // 3. HTTPS 实测（地址只从指定IP/DoH来，不走系统）
        sb.append("\n--- HTTPS 实测（DNS来源：DoH） ---\n")
        try {
            val dns = DohDns(dohServer, customIp, domain)
            val client = OkHttpClient.Builder()
                .dns(dns)
                .connectTimeout(15, TimeUnit.SECONDS)
                .readTimeout(15, TimeUnit.SECONDS)
                .followRedirects(true)
                .build()
            val req = Request.Builder().url("https://$domain/").header("User-Agent", "EchDemo/1.0").build()
            client.newCall(req).execute().use { resp ->
                sb.append("状态码: ${resp.code}\n")
                val altSvc = resp.header("alt-svc") ?: ""
                sb.append("alt-svc: ${if (altSvc.isEmpty()) "(空)" else altSvc}\n")
                if (altSvc.contains("h3")) sb.append("H3宣告: ✅ 服务端支持h3（真握手需二期QUIC库）\n")
                else sb.append("H3宣告: ❌ 未宣告h3\n")
                // 注意：本演示是 TCP 抓头，不是 QUIC 真连
                sb.append("注：本次为 TCP 抓头，非 QUIC 真连\n")
            }
        } catch (e: Exception) {
            sb.append("HTTPS失败: ${e.message}\n")
            if ((e.message ?: "").contains("reset", true)) {
                sb.append("提示：被重置多为线路污染，换指定IP重试\n")
            }
        }

        sb.append("\n===== 结束 =====")
        return sb.toString()
    }
}
