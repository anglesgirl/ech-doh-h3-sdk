// Android Gradle configuration for ech-doh-h3-sdk
// Add to your app/build.gradle or module build.gradle

// In your project's settings.gradle or root build.gradle, add the AAR as a flatDir repository
// repositories {
//     flatDir {
//         dirs 'libs'
//     }
// }

/*
// In your app/build.gradle:

dependencies {
    // Add the AAR file
    implementation(name: 'ech-doh-h3-sdk', ext: 'aar')
    
    // Or if using the jniLibs directly:
    // implementation fileTree(dir: 'src/main/jniLibs', include: ['*.so'])
    
    // Required dependencies for the SDK
    implementation "org.jetbrains.kotlin:kotlin-stdlib:1.9.0"
    implementation "androidx.core:core-ktx:1.12.0"
    
    // For network operations (if needed by the app)
    implementation "com.squareup.okhttp3:okhttp:4.12.0"
}

// In your AndroidManifest.xml, add internet permission:
/*
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
*/

// Kotlin usage example:

/*
import com.example.echdohh3sdk.Engine
import com.example.echdohh3sdk.EngineConfig
import com.example.echdohh3sdk.HttpMethod
import com.example.echdohh3sdk.HttpResponse

class NetworkManager {
    private val engine: Engine
    
    init {
        val config = EngineConfig(
            dohServer = "https://1.1.1.1/dns-query",
            dohBootstrapIp = null,
            connectTimeoutSecs = 10,
            requestTimeoutSecs = 30,
            enableH3 = true,
            enableEch = true,
            userAgent = "MyApp/1.0",
            verifyCertificates = true
        )
        engine = Engine.new(config)
    }
    
    suspend fun fetch(url: String): HttpResponse {
        return withContext(Dispatchers.IO) {
            engine.fetch(
                url = url,
                method = HttpMethod.GET,
                headers = mapOf("Accept" to "application/json"),
                body = byteArrayOf()
            )
        }
    }
    
    suspend fun post(url: String, body: ByteArray): HttpResponse {
        return withContext(Dispatchers.IO) {
            engine.fetch(
                url = url,
                method = HttpMethod.POST,
                headers = mapOf(
                    "Content-Type" to "application/json",
                    "Accept" to "application/json"
                ),
                body = body
            )
        }
    }
    
    fun shutdown() {
        engine.shutdown()
    }
}
*/