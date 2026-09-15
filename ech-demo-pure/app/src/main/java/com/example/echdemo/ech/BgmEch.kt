package com.example.echdemo.ech

import android.content.Context
import com.liar.han1meplus.EchHttpClient

// ECH 实测：与 Animeko 叉库同一套文件（包名除外），演示版先验。
object BgmEch {
    val protectedHosts: Set<String> = setOf("api.bgm.tv", "next.bgm.tv", "bgm.tv")

    fun isProtected(host: String): Boolean {
        val h = host.lowercase().trimEnd('.')
        return protectedHosts.any { h == it || h.endsWith(".$it") }
    }

    fun ensureInit(ctx: Context): Boolean {
        EchHttpClient.init(ctx.applicationContext)
        return EchHttpClient.isLoaded
    }

    // dohUrl：沿用用户填的 DoH；IP 字面量无需 bootstrap，域名由库内自行引导
    fun test(domain: String, dohUrl: String): String {
        val sb = StringBuilder()
        sb.append("--- ECH 实测 ---\n")
        if (!EchHttpClient.isLoaded) {
            sb.append("ECH 库未加载（仅支持 arm64），本次只做探测\n")
            return sb.toString()
        }
        try {
            val resp = EchHttpClient.execute(
                "GET", "https://$domain/",
                mapOf("User-Agent" to "EchDemo/1.0"),
                null, dohUrl, "",
            )
            sb.append("状态码: ${resp.statusCode}\n")
            sb.append("ECH: ${resp.echStatus}\n")
            sb.append("回包字节: ${resp.body.size}\n")
            sb.append(if (resp.echStatus.contains("accepted", true)) "结论: ✅ 加密握手成功\n" else "结论: ❌ 未确认加密成功\n")
        } catch (e: Exception) {
            sb.append("ECH 请求失败: ${e.message}\n")
        }
        return sb.toString()
    }
}
