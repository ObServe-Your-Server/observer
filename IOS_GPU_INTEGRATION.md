# Getting GPU metrics into the iOS app

This is for whoever is wiring the new GPU data into the iOS app. I wrote the agent side, and I want to save you the afternoon I'd otherwise expect you to lose on it. Most of that time would go to one wrong assumption about which side of the connection is the server, so I'll start there.

Short version:

- The agent has never listened on a port, and it still doesn't. It dials **out** to whatever `base_server_url` points at, opens one long-lived bidirectional gRPC stream, and then answers questions sent down that stream.
- "Connecting directly to the GPU service" therefore means the reverse of what you'd expect: **your app (or Watch-Tower) is the gRPC server, and the agent connects to it.**
- GPU data is one more `MetricsType` (`GPU = 9`) on the existing `MetricsTunnel.BaseTransfer` RPC. It's also included in `FULL` responses. There is no new service and no new RPC.
- Every GPU field except `name` is optional, and the agent really does leave them unset depending on the platform. Treat "absent" as "unknown", never as zero.

---

## 1. Which way the connection goes

```
                       ┌──────────────────────────────┐
  observer agent ────► │  MetricsTunnel server        │
  (on the box)  dials  │  (Watch-Tower today, or      │
                 out   │   your iPhone in dev mode)   │
                       └──────────────────────────────┘

  BaseTransfer(stream MetricsResponse) returns (stream MetricsRequest)

  server ──MetricsRequest{ request_id, type: GPU, last_n: 1 }──► agent
  server ◄──MetricsResponse{ request_id, gpu_metrics: [...] }─── agent
```

Look at `BaseTransfer` in `bins/client/proto/observer/v1/metrics_tunnel.proto`. The agent is the gRPC *client*, but it's the side that *sends responses*. The server pushes `MetricsRequest`s down its half of the stream, and the agent replies on its half, matched by `request_id`. You can see this in `bins/client/src/grpc/v1/metrics_tunnel.rs`, in `connect_and_serve`. The agent builds a `MetricsTunnelClient`, attaches `x-api-key` metadata, and then loops over inbound requests and calls `build_response` for each one.

That leaves you two real ways to get GPU data into the app:

| | Path A: through Watch-Tower | Path B: iPhone is the tunnel server |
|---|---|---|
| What it is | Production. The backend holds the tunnel and the app asks the backend | Dev/LAN. You point one agent's `base_server_url` at the phone |
| Who needs proto changes | Watch-Tower and the app's API to it | Only the app |
| Works off-LAN / in background | Yes | No. iOS suspends the listener when the app backgrounds |
| Good for | Shipping | Building and testing the GPU UI today, without waiting on the backend |

I'd build the UI against Path B first, because it needs nothing from anyone else, and ship on Path A. The decoding code is identical either way, since both paths end with the app holding a `GpuMetricsList`.

---

## 2. What's on the wire

### The messages

`bins/client/proto/observer/v1/metrics/gpu.proto`:

```proto
message GpuEntry {
  repeated GpuMetrics gpus = 1;              // one per GPU found in that pass
  google.protobuf.Timestamp collected_at = 2;
}

message GpuMetrics {
  string name = 1;
  optional float  usage_percent       = 2;   // 0–100
  optional uint64 memory_used_bytes   = 3;
  optional uint64 memory_total_bytes  = 4;
  optional float  temperature_celsius = 5;
  google.protobuf.Timestamp collected_at = 6; // same instant as the entry's
}
```

And these additions to `metrics_tunnel.proto`:

```proto
enum MetricsType { ... FULL = 8; GPU = 9; }
message MetricsResponse { oneof response { ... GpuMetricsList gpu_metrics = 11; } }
message GpuMetricsList { repeated observer.v1.metrics.GpuEntry items = 1; }
message FullMetrics { ... GpuMetricsList gpu_metrics = 9; }
```

Everything is purely additive with fresh field numbers, so nothing that already decodes these messages breaks.

### How a request is interpreted

This is what `build_response` / `QueryRange::from_request` actually does. Some of it isn't obvious from the proto:

| You send | You get |
|---|---|
| `type: GPU, last_n: N` | The newest N collection passes, **oldest first**. Each pass is one `GpuEntry`. |
| `type: GPU, range: {start, end}` | Every pass with `start ≤ collected_at ≤ end`, oldest first. Unix **seconds**, inclusive on both ends. |
| `type: GPU`, no `query` set | Effectively nothing. The range collapses to `[now, now]`. Always set one. |
| `type: FULL, last_n: N` | N passes **per metric**, with GPU in `full_metrics.gpu_metrics`. |
| A type the agent doesn't know | The agent quietly treats it as `FULL`. See the version gotcha in §6. |
| Anything, when the agent's DB query fails | A response with your `request_id` and **no `response` oneof set**. It doesn't error and it doesn't drop the stream. |

### How a pass becomes a `GpuEntry`

- The agent collects every `metric_secs` seconds (2–60, default 5). One collection pass produces one `GpuEntry`, with one `GpuMetrics` per GPU.
- If a pass finds zero GPUs, **nothing is stored**: no empty entry. So on a box without a GPU, `last_n: 1` returns an empty `items` array. That's how you detect "no GPU here".
- History is kept for `metrics_retention_time_hours` (24 by default). Older entries are deleted along with their GPU rows.

### Which fields you'll actually get, per platform

This matters for the UI more than anything else in this doc:

| Agent host | Backend | name | usage % | VRAM used | VRAM total | temp |
|---|---|:-:|:-:|:-:|:-:|:-:|
| Linux / Windows, NVIDIA w/ driver | NVML | ✓ | ✓ | ✓ | ✓ | ✓ |
| Linux, AMD | sysfs (`amdgpu`) | ✓ | ✓ | ✓ | ✓ | ✓ (hwmon) |
| Linux, Intel | sysfs (`i915`) | ✓ | — (needs root PMU) | — | — | usually — |
| Linux, NVIDIA on nouveau | sysfs | ✓ | — | — | — | maybe |
| Windows, any non-NVIDIA | DXGI + PDH | ✓ | ✓ | ✓ | ✓ | — |
| macOS, Apple Silicon | `ioreg` | ✓ | ✓ | ✓ (shared system memory in use) | — | — |
| macOS, Intel Mac w/ dGPU | `ioreg` | ✓ | ✓ | ✓ | ✓ | — |

On Linux an Intel iGPU really does show up as just a name. I saw exactly that on my own box next to a fully populated RTX 4070 Ti. The card should look intentional in that state, not broken. On Apple Silicon `memory_used_bytes` is unified memory, so don't draw a "VRAM x / y" bar with a missing denominator. Show the absolute number.

---

## 3. Path B: making the iPhone the tunnel server

This is the "direct" route. The app runs a gRPC server on the LAN (or tailnet), you edit one line in an agent's config, and the agent connects to your phone instead of Watch-Tower.

### 3.1 Generate Swift code from the protos

Use the protos in `bins/client/proto` as-is. Don't hand-copy messages into Swift. You need `swift-protobuf` for the messages and `grpc-swift` 2.x for the service.

Packages (SwiftPM):

- `grpc-swift` 2.x: `GRPCCore`
- `grpc-swift-nio-transport`: `GRPCNIOTransportHTTP2`
- `grpc-swift-protobuf`: `GRPCProtobuf` (plus its protoc plugin)
- `swift-protobuf`: `SwiftProtobuf`

grpc-swift 2 requires iOS 18 / Swift 6. If the app has to support anything older, use grpc-swift 1.x instead. The concepts below carry over unchanged, but the generated API names are different.

Codegen from the repo root:

```sh
mkdir -p Generated
protoc -I bins/client/proto \
  --swift_out=Visibility=Public:Generated \
  --grpc-swift-2_out=Visibility=Public,Server=true,Client=false:Generated \
  observer/v1/metrics_tunnel.proto observer/v1/metrics/*.proto
```

The plugin binary has been renamed between releases (`protoc-gen-grpc-swift-2` vs the older `protoc-gen-grpc-swift`, flag `--grpc-swift_out`). Use whichever one your `grpc-swift-protobuf` version installs. You can also skip the manual step and use that package's SwiftPM build plugin. `google/protobuf/timestamp.proto` ships with protoc, so it resolves without extra include paths.

The generated names you'll actually touch:

| Proto | Swift |
|---|---|
| `observer.v1.MetricsRequest` | `Observer_V1_MetricsRequest` (`.requestID`, `.type`, `.query`) |
| `query` oneof | `.range(Observer_V1_TimeRange)` / `.lastN(Int64)` |
| `MetricsType.GPU` / `FULL` | `.gpu` / `.full` |
| `MetricsResponse.response` oneof | `.gpuMetrics(Observer_V1_GpuMetricsList)`, `.fullMetrics(...)` |
| `observer.v1.metrics.GpuEntry` | `Observer_V1_Metrics_GpuEntry` |
| `GpuMetrics.usage_percent` | `.usagePercent` + **`.hasUsagePercent`** |
| `collected_at` | `.collectedAt.date` (`Google_Protobuf_Timestamp`) |

That `has…` accessor is the reason the proto uses `optional`. The plain property returns `0` when the field is unset, which happens to be a perfectly plausible GPU utilisation. Always check `has…` first.

### 3.2 The service

The agent opens one stream and keeps it open. Your job is to hold on to the writer for that stream, send requests whenever the UI wants data, and route each response back to whoever asked by `request_id`.

```swift
import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2
import SwiftProtobuf

actor AgentSession {
    let apiKey: String
    private let writer: RPCWriter<Observer_V1_MetricsRequest>
    private var waiting: [String: CheckedContinuation<Observer_V1_MetricsResponse, any Error>] = [:]

    init(apiKey: String, writer: RPCWriter<Observer_V1_MetricsRequest>) {
        self.apiKey = apiKey
        self.writer = writer
    }

    func ask(_ type: Observer_V1_MetricsType,
             _ query: Observer_V1_MetricsRequest.OneOf_Query) async throws -> Observer_V1_MetricsResponse {
        var request = Observer_V1_MetricsRequest()
        request.requestID = UUID().uuidString
        request.type = type
        request.query = query

        return try await withCheckedThrowingContinuation { continuation in
            waiting[request.requestID] = continuation
            Task { await self.write(request) }
        }
    }

    private func write(_ request: Observer_V1_MetricsRequest) async {
        do {
            try await writer.write(request)
        } catch {
            waiting.removeValue(forKey: request.requestID)?.resume(throwing: error)
        }
    }

    func deliver(_ response: Observer_V1_MetricsResponse) {
        waiting.removeValue(forKey: response.requestID)?.resume(returning: response)
    }

    func close(_ error: any Error) {
        for continuation in waiting.values { continuation.resume(throwing: error) }
        waiting.removeAll()
    }
}

@MainActor
final class AgentHub: ObservableObject {
    @Published private(set) var session: AgentSession?
    func attach(_ session: AgentSession) { self.session = session }
    func detach(_ session: AgentSession) { if self.session === session { self.session = nil } }
}

struct MetricsTunnelService: Observer_V1_MetricsTunnel.ServiceProtocol {
    let hub: AgentHub
    let expectedKey: String

    func baseTransfer(
        request: StreamingServerRequest<Observer_V1_MetricsResponse>,
        context: ServerContext
    ) async throws -> StreamingServerResponse<Observer_V1_MetricsRequest> {
        let key = Array(request.metadata[stringValues: "x-api-key"]).first ?? ""
        guard key == expectedKey else {
            throw RPCError(code: .unauthenticated, message: "bad api key")
        }

        return StreamingServerResponse { writer in
            let session = AgentSession(apiKey: key, writer: writer)
            await hub.attach(session)
            do {
                for try await message in request.messages {
                    await session.deliver(message)
                }
                await session.close(CancellationError())
            } catch {
                await session.close(error)
            }
            await hub.detach(session)
            return [:]
        }
    }
}
```

A few choices in there are deliberate:

- **The metadata key is `x-api-key`.** That's what the agent sends. There's an older Rust server stub in `bins/client/src/grpc/v1/metrics/server.rs` that reads `api_key` (no `x-`), and so does the ignored test in `metrics_tunnel.rs`. Don't copy from either. They don't match what the real agent sends.
- **Rejecting with `.unauthenticated` is final.** The agent treats `Unauthenticated`, `PermissionDenied` and `InvalidArgument` as non-retryable, and the whole process exits. systemd then restarts it 60 s later (`Restart=on-failure`, `RestartSec=60s`). If you fat-finger the key you'll see that cycle in `journalctl -u observer`, not a tight retry loop.
- **The producer closure never returns while the agent is connected.** Returning from it ends the stream. The agent then reconnects after 5 s, and if it can't get a stable connection for 5 minutes it exits (and gets restarted as above).
- I've left out request timeouts for brevity. Add one (race `ask` against a `Task.sleep`) before relying on it. A request the agent never answers would otherwise hang its caller forever.

### 3.3 Starting the listener

```swift
@MainActor
final class AgentListener {
    private var task: Task<Void, Never>?

    func start(hub: AgentHub, apiKey: String, port: Int = 50051) {
        task = Task {
            let server = GRPCServer(
                transport: .http2NIOPosix(
                    address: .ipv4(host: "0.0.0.0", port: port),
                    transportSecurity: .plaintext
                ),
                services: [MetricsTunnelService(hub: hub, expectedKey: apiKey)]
            )
            do { try await server.serve() } catch { print("tunnel server stopped: \(error)") }
        }
    }

    func stop() { task?.cancel() }
}
```

Plaintext is fine here. The agent speaks h2c (HTTP/2 with prior knowledge) whenever the URL starts with `http://`, and switches to TLS with native roots on `https://`. If you want TLS on the phone, the agent has to trust your cert through the system store, which isn't worth the effort for a dev setup.

Two more things before it works:

- **Info.plist:** add `NSLocalNetworkUsageDescription`. Local Network privacy covers inbound connections too. Without the permission the agent's connection never reaches you, and nothing in the app tells you why.
- **ATS doesn't apply.** ATS governs connections the app *makes*, not ones it accepts, so no exception is needed for the plaintext listener.

### 3.4 Point an agent at the phone

On the box running the agent, edit `/etc/observer/observer.toml`. Note that `observer.toml.example` in the repo root is out of date: the agent doesn't read `base_metrics_url`, and it requires `database_url` and `metrics_retention_time_hours`. This is the shape it actually parses (see `setup/deploy-pre.sh`):

```toml
[server]
base_server_url   = "http://192.168.1.50:50051"
base_notifier_url = "https://watch-tower.observe.vision/v1/ingest/notification"
database_url      = "sqlite:///var/lib/observer/observer.db?mode=rwc"
api_key           = "dev-key-you-also-put-in-the-app"
metrics_retention_time_hours = 24

[intervals]
metric_secs          = 5
speedtest_secs       = 300
enable_docker_socket = true
docker_secs          = 10
```

Then run `sudo systemctl restart observer && journalctl -u observer -f`. When the app's listener is up, you should see `Connected to the server at http://192.168.1.50:50051` in the log, and `hub.session` becomes non-nil in the app.

Use the phone's Wi-Fi IP. A tailnet IP should also work if both ends are on Tailscale, but I haven't tested that path. Keep the app in the foreground the whole time: once iOS suspends it, the socket goes with it.

You don't need a phone for the first pass either. The service and listener above compile unchanged in a macOS command-line target, so you can point the agent at your Mac, get the decoding right in a terminal, and then move it into the app.

---

## 4. Asking for GPU data

Once a session exists, you only need two calls.

**Live gauge (latest pass):**

```swift
let response = try await session.ask(.gpu, .lastN(1))
guard case .gpuMetrics(let list)? = response.response else {
    // nil  -> agent-side query failed; retry next tick
    // .fullMetrics -> agent predates GPU support (see §6)
    return
}
let latest = list.items.last   // nil => this host has no GPU
```

**History for a chart:**

```swift
let now = Int64(Date().timeIntervalSince1970)
var window = Observer_V1_TimeRange()
window.start = now - 3600
window.end = now
let response = try await session.ask(.gpu, .range(window))
```

Then map the proto into something the views can use, and do the `has…` checks exactly once, at this boundary:

```swift
struct GPUReading: Identifiable, Hashable {
    let id: String
    let name: String
    let usage: Double?
    let memoryUsed: UInt64?
    let memoryTotal: UInt64?
    let temperature: Double?
    let at: Date
}

extension Observer_V1_Metrics_GpuEntry {
    func readings() -> [GPUReading] {
        let at = hasCollectedAt ? collectedAt.date : .distantPast
        return gpus.enumerated().map { index, gpu in
            GPUReading(
                id: "\(index):\(gpu.name)",
                name: gpu.name,
                usage: gpu.hasUsagePercent ? Double(gpu.usagePercent) : nil,
                memoryUsed: gpu.hasMemoryUsedBytes ? gpu.memoryUsedBytes : nil,
                memoryTotal: gpu.hasMemoryTotalBytes ? gpu.memoryTotalBytes : nil,
                temperature: gpu.hasTemperatureCelsius ? Double(gpu.temperatureCelsius) : nil,
                at: at
            )
        }
    }
}
```

### Polling cadence

The tunnel is request/response only, so nothing gets pushed to you. Poll `last_n: 1` at the agent's `metric_secs` (5 s unless someone changed it). Polling faster just returns the same entry again, so dedupe on `collected_at` rather than trusting your timer.

The agent answers requests **one at a time per connection**: it awaits each DB query before it reads the next request. A big `FULL` history request will therefore hold up your GPU gauge until it finishes. Ask for `GPU` on its own when that's all a screen needs, and avoid fetching `FULL` on the same tick as live updates.

### Payload size

24 h of history at 5 s is 17,280 entries. For one or two GPUs that's a couple of MB, which is close to the 4 MiB default receive limit most gRPC servers ship with, and the limit applies on *your* side here because the agent is the one sending. For charts, ask for the window on screen, not the whole retention period. If you really need everything, raise the max request payload size in the server transport config.

---

## 5. Path A: through Watch-Tower

Path A doesn't change the agent at all. Whoever owns Watch-Tower needs to:

1. Regenerate their stubs from the updated `metrics_tunnel.proto` and `metrics/gpu.proto`.
2. Either send `MetricsRequest{ type: GPU }` down the existing tunnel, or read `full_metrics.gpu_metrics` from the `FULL` requests they already make. The second option costs zero new round-trips.
3. Expose it to the app however the app already gets CPU/memory, with the same optional-means-unknown semantics. If that API is JSON, use `null` for absent fields, never `0`.

The app-side mapping is the `GPUReading` code above, just fed from the backend's response.

Until Watch-Tower does steps 1–2, a new agent's GPU data travels in `FULL` responses as unknown field 9 and gets silently dropped. Nothing breaks, but nothing shows up either.

---

## 6. Gotchas, collected

- **Old agents answer a GPU request with FULL data.** An agent built before this change can't parse `type = 9`, and `build_response` falls back to `FULL` for any type it doesn't recognise. So a `.gpu` request to an old agent returns `.fullMetrics`, which has no GPU field. Treat "asked for GPU, got FULL" as "agent needs updating", and show that in the UI instead of an empty card.
- **No stable GPU id.** GPUs are identified by `name` and position in the list. Two identical cards produce the same name. Order is stable for a given host in practice (NVML devices first, then sysfs/DXGI), so `index:name` works as a key. If you hit a case where it doesn't, the clean fix is a `string id = 7;` carrying the NVML UUID / PCI slot, which the collector already has. Tell me and I'll add it.
- **Two timestamps.** `GpuEntry.collected_at` and each `GpuMetrics.collected_at` are the same instant. Use the entry's.
- **Empty `items` ≠ error.** It means no GPU (or no pass in that window). A response with an unset oneof means a query failure on the agent.
- **Ranges are seconds, not milliseconds.** Pass `Date().timeIntervalSince1970 * 1000` and you'll get an empty list with no error.
- **The repo's `observer.toml.example` is stale.** Use the shape in §3.4.

---

## 7. What's verified and what isn't

To be upfront about where this stands:

- **Verified on a Linux box** (RTX 4070 Ti + Intel iGPU): the collector's numbers match `nvidia-smi`. `cargo test --workspace` passes. A temporary end-to-end test (migrate → three collection passes → `GPU` and `FULL` responses → retention cleanup) passed before I removed it.
- **Not verified:** the Windows and macOS backends were never compiled, because I had no cross toolchain. No real Swift client has talked to this yet, so the Swift in this doc is written against grpc-swift 2's API, not copied from a running app. Expect to fix a name or two when it first compiles. The wire behaviour described above is what the Rust code actually does.

If something on the wire doesn't match this doc, trust `build_response` in `bins/client/src/grpc/v1/metrics_tunnel.rs`, and let me know so I can fix the doc.
