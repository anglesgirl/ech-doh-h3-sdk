// iOS Integration Guide for ech-doh-h3-sdk
// Add the XCFramework to your Xcode project

/*
1. Drag and drop ech_doh_h3_sdk.xcframework into your Xcode project
   - Check "Copy items if needed"
   - Add to your target's "Frameworks, Libraries, and Embedded Content"
   - Set to "Embed & Sign"

2. In your target's Build Settings:
   - Enable "Allow Non-modular Includes In Framework Modules" = YES
   - Add $(PROJECT_DIR)/ech_doh_h3_sdk.xcframework/ios-arm64/Headers to Header Search Paths (if needed)

3. In Info.plist, add:
   <key>NSAppTransportSecurity</key>
   <dict>
       <key>NSAllowsArbitraryLoads</key>
       <true/>
   </dict>

4. Swift usage example:
*/

import Foundation
import ech_doh_h3_sdk

class NetworkManager {
    private let engine: Engine
    
    init() {
        let config = EngineConfig(
            dohServer: "https://1.1.1.1/dns-query",
            dohBootstrapIp: nil,
            connectTimeoutSecs: 10,
            requestTimeoutSecs: 30,
            enableH3: true,
            enableEch: true,
            userAgent: "MyApp/1.0",
            verifyCertificates: true
        )
        self.engine = Engine.new(config: config)!
    }
    
    func fetch(url: String) async throws -> HttpResponse {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    let response = try self.engine.fetch(
                        url: url,
                        method: .GET,
                        headers: ["Accept": "application/json"],
                        body: Data()
                    )
                    continuation.resume(returning: response)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
    
    func post(url: String, body: Data) async throws -> HttpResponse {
        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                do {
                    let response = try self.engine.fetch(
                        url: url,
                        method: .POST,
                        headers: [
                            "Content-Type": "application/json",
                            "Accept": "application/json"
                        ],
                        body: body
                    )
                    continuation.resume(returning: response)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }
    
    func shutdown() {
        engine.shutdown()
    }
}

// Usage:
/*
let networkManager = NetworkManager()

Task {
    do {
        // GET request
        let response = try await networkManager.fetch(url: "https://example.com/api/data")
        print("Status: \(response.statusCode)")
        print("Headers: \(response.headers)")
        print("Body: \(String(data: response.body, encoding: .utf8) ?? "nil")")
        
        // POST request
        let jsonData = try JSONEncoder().encode(["key": "value"])
        let postResponse = try await networkManager.post(url: "https://example.com/api/submit", body: jsonData)
        print("POST Status: \(postResponse.statusCode)")
    } catch {
        print("Error: \(error)")
    }
    
    // Cleanup
    networkManager.shutdown()
}
*/