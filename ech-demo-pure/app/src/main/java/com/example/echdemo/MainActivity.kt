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
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val etDomain = findViewById<EditText>(R.id.etDomain)
        val etDoh = findViewById<EditText>(R.id.etDohServer)
        val btnTest = findViewById<Button>(R.id.btnTest)
        val tvResult = findViewById<TextView>(R.id.tvResult)

        btnTest.setOnClickListener {
            val domain = etDomain.text.toString().trim()
            if (domain.isEmpty()) {
                Toast.makeText(this, "请输入域名", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            val doh = etDoh.text.toString().trim().ifEmpty { "https://1.1.1.1/dns-query" }
            tvResult.text = "测试中...\n域名: $domain\nDoH: $doh"
            btnTest.isEnabled = false
            CoroutineScope(Dispatchers.IO).launch {
                val out = runTest(domain, doh)
                withContext(Dispatchers.Main) {
                    tvResult.text = out
                    btnTest.isEnabled = true
                }
            }
        }
    }

    private fun httpGet(urlStr: String, accept: String? = null): Pair<Int, String> {
        val c = (URL(urlStr).openConnection() as HttpURLConnection).apply {
            connectTimeout = 15000
            readTimeout = 15000
            requestMethod = "GET"
            setRequestProperty("User-Agent", "EchDemo/1.0")
            if (accept != null) setRequestProperty("Accept", accept)
        }
        return try {
            val code = c.responseCode
            val s = try {
                c.inputStream.bufferedReader().readText()
            } catch (e: Exception) {
                c.errorStream?.bufferedReader()?.readText() ?: ""
            }
            // 取关键头
            val altSvc = c.getHeaderField("alt-svc") ?: ""
            Pair(code, "$s\n[ALT-SVC:$altSvc]")
        } finally {
            c.disconnect()
        }
    }

    private fun runTest(domain: String, dohServer: String): String {
        val sb = StringBuilder()
        sb.append("===== 测试结果 =====\n域名: $domain\nDoH: $dohServer\n\n")

        // 1. DoH 查 HTTPS 记录
        sb.append("--- DoH HTTPS 记录 ---\n")
        try {
            val q = "$dohServer?name=$domain&type=HTTPS"
            val (code, body) = httpGet(q, "application/dns-json")
            sb.append("DoH状态: $code\n")
            if (code == 200) {
                val json = JSONObject(body.substringBefore("[ALT-SVC:"))
                val answers = json.optJSONArray("Answer")
                if (answers == null) {
                    sb.append("无Answer: 可能不支持HTTPS记录\n")
                } else {
                    var hasH3 = false
                    var hasEch = false
                    for (i in 0 until answers.length()) {
                        val data = answers.getJSONObject(i).optString("data", "")
                        sb.append("记录$i: $data\n")
                        if (data.contains("h3")) hasH3 = true
                        if (data.contains("ech=")) hasEch = true
                    }
                    sb.append("H3: ${if (hasH3) "✅ 广告h3" else "❌ 未见h3"}\n")
                    sb.append("ECH: ${if (hasEch) "✅ 含ech=" else "❌ 未见ech="}\n")
                }
            } else {
                sb.append("DoH查询失败\n")
            }
        } catch (e: Exception) {
            sb.append("DoH异常: ${e.message}\n")
        }

        // 2. 直接 HTTPS 抓头看 alt-svc
        sb.append("\n--- HTTPS 实测 ---\n")
        try {
            val (code, body) = httpGet("https://$domain")
            val altSvc = body.substringAfter("[ALT-SVC:", "").substringBefore("]", "")
            sb.append("状态码: $code\n")
            sb.append("alt-svc: ${if (altSvc.isEmpty()) "(空)" else altSvc}\n")
            if (altSvc.contains("h3")) sb.append("H3实测: ✅ 服务端宣告h3\n")
            else sb.append("H3实测: ⚠️ 未宣告h3\n")
        } catch (e: Exception) {
            sb.append("HTTPS失败: ${e.message}\n")
        }

        sb.append("\n===== 结束 =====")
        return sb.toString()
    }
}
