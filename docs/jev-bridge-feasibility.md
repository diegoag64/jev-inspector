# A standalone Jev inspector with a local bridge

Research date: 2026-11-06. This is a feasibility assessment and proposed design.
The accompanying experiment validates selected SDK behavior against local mocks.
It does not implement the Rust product or validate live TypeSafe service behavior.

The idea is feasible. Backend applications can direct their existing TypeSafe
clients to a local HTTP service, which forwards calls to TypeSafe and returns the
responses. A Rune application can inspect a separate feed of captured events.
The useful product is a Jev debugger that works across backend languages, with
optional application instrumentation for workflow context.

## What the existing project already does

The current jev-devtools capture mechanism already runs on the Node backend.
React supplies the inspector. The server adapter wraps `systemOne`, uses
`AsyncLocalStorage` to associate transport attempts with one logical call, and
publishes bounded in-memory history through SSE. Its restrictions are the
instrumented JavaScript client and process-local history, rather than an inability
to observe backend calls. See the pinned [architecture](https://github.com/diegoag64/jev-devtools/blob/b6ccfdd83729235fbfa0ddbea77d89166621fccc/docs/architecture.md)
and [recorder](https://github.com/diegoag64/jev-devtools/blob/b6ccfdd83729235fbfa0ddbea77d89166621fccc/packages/jev-devtools/server/recorder.ts).

The Flow runtime adds information that HTTP does not contain: authored definitions,
steps, branches, task execution, loops, and run relationships. These describe the
application's execution, not hidden reasoning inside Jev. A bridge can preserve
these views if the application sends that information separately.

There is also a browser viewer export, `jev-devtools/viewer`, with
`useJevInspection` and `JevFlowViewer`. The hook expects a same-origin inspector
endpoint. A standalone web portal could reuse this code if the service implements
the existing stream protocol or a compatible adapter. This is useful reuse, but
does not make the current package a generic HTTP proxy. See the
[viewer source](https://github.com/diegoag64/jev-devtools/blob/b6ccfdd83729235fbfa0ddbea77d89166621fccc/packages/jev-devtools/src/viewer.tsx)
and [stream protocol](https://github.com/diegoag64/jev-devtools/blob/b6ccfdd83729235fbfa0ddbea77d89166621fccc/docs/stream-protocol.md).

## The integration point is supported

The official JavaScript SDK accepts `baseURL`; Python accepts `base_url`.
Both also read `TYPESAFE_BASE_URL`, with explicit constructor settings taking
precedence. They append endpoint paths such as `/v1/systemone` to that base.
See the pinned [JavaScript client](https://github.com/typesafe-ai/typesafe-sdk-js/blob/b844cd1aca12855a5d24a2df52a1401c86e7d37a/src/client.ts)
and [Python configuration](https://github.com/typesafe-ai/typesafe-sdk-python/blob/f36e68549ceca3191e5791e5f99ea1baa1a258b5/src/typesafe_sdk/_core/config.py).

For a future bridge listening on port 8787, a backend launched from PowerShell
could use:

```powershell
$env:TYPESAFE_BASE_URL = "http://127.0.0.1:8787"
# Start the backend in this environment, keeping its existing TypeSafe API key.
```

On macOS, the equivalent process environment is:

```sh
export TYPESAFE_BASE_URL=http://127.0.0.1:8787
```

These are proposed integration examples. No persistent bridge has been installed.
The base should not include `/v1`, because the SDK supplies it. Clients that
hardcode their URL need a configuration change. Other languages need an HTTP
client or SDK that can target this address; they do not need Rust or React.

This is an explicit reverse proxy. The application deliberately connects to it,
and the proxy establishes its own HTTPS connection to TypeSafe. It does not need
a root certificate installation or interception of arbitrary HTTPS traffic.
The [mitmproxy modes documentation](https://docs.mitmproxy.org/stable/concepts/modes/)
describes the distinction. Inference still runs at TypeSafe; a local inspector
does not make cloud inference offline.

## Evidence from a local compatibility experiment

The [reproducible probe](../research/README.md) uses the published JavaScript SDK
0.7.0 and Python SDK 0.7.5, with a fake key and synthetic data. All traffic stays
on IPv4 loopback. The mock upstream and test relay close when the run finishes.

Seventeen scenarios passed, generating 23 HTTP requests:

- JavaScript explicit base URL and Python environment-variable routing.
- JSON and multipart image requests from both SDKs.
- A zstd-compressed Python request and a large JavaScript JSON request.
- Gzip response decoding, typed answers, and SDK access to the upstream request ID.
- Model listing and 401 errors without retries.
- One 529 followed by success, requiring two attempts in each SDK.
- Persistent 429 errors, requiring three attempts in each SDK under defaults.
- Python asynchronous requests as well as synchronous requests.

Request and response body SHA-256 values matched between the relay and mock
upstream for every exchange. The TypeSafe request-ID header is
`x-typesafe-request-id`, not `x-request-id`. The probe also verified the image
opt-in header and the image bytes reaching the mock.

The local evidence file is `artifacts/research/bridge-probe.json`. The relay is
small JavaScript research code that buffers bodies, with a 1 MiB test limit.
It is not a streaming Rust implementation or a benchmark.

The JavaScript run used the tool's Node execution environment. That environment
does not expose normal Node process detection to the SDK, so automatic JavaScript
zstd compression was disabled. Python zstd was tested successfully. A normal Node
run, live TypeSafe calls, TLS, HTTP/2, cancellation, load, and macOS execution remain
unverified. A portable Node download failed its published checksum and was removed
without execution; it was not used as evidence. The CLI wrapper and setup commands
are supplied for subsequent runs.

## What the bridge can know

| Capability | HTTP bridge alone | Additional information needed |
| --- | --- | --- |
| State, question definitions, answer types and returned probabilities | Yes, when captured bodies are complete and decodable | A parser for the observed API format |
| HTTP status, body sizes, upstream request ID and observed elapsed time | Yes | Separate timing definitions for arrival, upstream response and completion |
| Model returned and token usage | Yes, when the response contains them | Pricing version if showing a cost estimate |
| Which service or environment made a request | Sometimes configured by connection or listener | Explicit service/session labels for reliable identification |
| One logical SDK call and all its attempts | No reliable grouping from identical bodies | A stable call ID supplied by instrumentation |
| Application tasks, branches, skipped nodes and Flow definitions | No | Flow events or spans emitted by the application |
| Validation failures before sending, cache hits, or SDK parsing failures after receipt | No | Application or SDK instrumentation |
| Why the model reached a decision internally | No | Not provided by the API |

An incoming HTTP request should be recorded as an observed exchange. Three equal
requests might be retries or three intentional calls. Do not infer a relationship
from timing and identical bodies and then present it as fact.

The proposed event model should distinguish `proxy_exchange`, `sdk_call`, and
`flow_step`, linking them only when explicit IDs exist. Preserve source, schema
version, timestamps, fidelity, truncation and redaction metadata. An adapter can
project compatible fields into the existing viewer protocol; unknown logical-call
metadata must remain unknown.

Optional instrumentation can propagate W3C trace context and emit local steps.
[OpenTelemetry context propagation](https://opentelemetry.io/docs/concepts/context-propagation/)
provides a standard basis for cross-service relationships. Correlation headers
intended only for the inspector should be consumed locally instead of leaking
application metadata upstream. Supporting OTLP requires a real receiver and a
mapping to Jev/Flow concepts; it is a separate feature.

## Recommended process structure

The backend sends requests to a small Rust daemon. The daemon streams them to a
fixed TypeSafe upstream and returns responses. A bounded capture branch feeds
decoding, redaction, local storage, and inspector subscriptions. The Rune app reads
that subscription and queries stored history. It can close and reconnect while
the daemon continues serving traffic.

A reasonable Rust implementation uses Tokio with Axum/Hyper for the listener,
an explicitly configured upstream HTTP client, and SQLite for a local history.
These are design recommendations, not components already built in this repository.

Keep the daemon in a crate that has no Rune or GPU dependency. Share event types
and configuration through another small crate. The desktop app can launch the
daemon for development convenience, but process ownership and shutdown behavior
must be explicit. Hidden windows, GPU initialization failures, or an inspector
crash must not stop an otherwise healthy relay.

Use separate access controls for inference traffic, inspector reads, and actions
such as replay or clearing history. An inspector endpoint must never accidentally
become a public request-forwarding endpoint.

Rune fits the desktop client. Its kit includes a virtualized table, sorting,
resizable columns, tree controls, inputs, dialogs, and syntax-oriented views.
`AppHandle::wake` can wake the UI when a background worker queues updates.
See the pinned [table](https://github.com/anomalyco/rune/blob/781e2b6d87cd5f32d6f4365956fad4baaf24ee50/crates/rune-kit/src/table.rs)
and [application API](https://github.com/anomalyco/rune/blob/781e2b6d87cd5f32d6f4365956fad4baaf24ee50/crates/rune/src/app.rs).

The application still needs Jev-specific question/answer rendering, filters,
comparison views, payload limits, and any interactive Flow graph. React components
cannot be dropped into Rune. Reusing protocol definitions, fixtures, UX decisions,
and capture-fidelity rules saves work; porting the UI is real implementation work.

Rune identifies itself as pre-release and asks consumers to pin a revision. It
targets macOS, Windows, Linux, and web. That does not establish support for every
device or equal maturity across platforms. The existing QA setup has Windows
evidence; it does not validate a future inspector on macOS, Linux, mobile, or web.
See the [pinned Rune README](https://github.com/anomalyco/rune/blob/781e2b6d87cd5f32d6f4365956fad4baaf24ee50/README.md).

If a browser portal becomes important, the daemon API can serve both a native
client and a React viewer. The existing viewer is the most direct reuse path.
Rune's web target is another candidate to evaluate, but a browser client cannot
replace the backend HTTP daemon that listens for and forwards these requests.

## Protocol details that determine correctness

The documented API has `POST /v1/systemone` and `GET /v1/models`. Successful
inference responses are complete JSON objects. The devtools SSE stream is a
separate inspection feed; it is not model token streaming. The authoritative
endpoint schemas are in the [TypeSafe OpenAPI document](https://api.typesafe.ai/openapi.json).

The daemon should forward HTTP body bytes without parsing and reserializing them
on the critical path. It can decode a bounded copy for display. This preserves
numeric/text representation, multipart boundaries and image bytes. Forwarding
success must not depend on the inspector understanding a newly added field.

The official SDKs can compress bodies larger than their 1024-byte threshold with
zstd when their runtime supports it. Python before 3.14 needs its optional zstd
dependency. Multipart image requests are another valid format. The bridge needs
separate forwarding and inspection limits, with explicit records when inspection
is incomplete. See the pinned [JavaScript client](https://github.com/typesafe-ai/typesafe-sdk-js/blob/b844cd1aca12855a5d24a2df52a1401c86e7d37a/src/client.ts),
[Python compression module](https://github.com/typesafe-ai/typesafe-sdk-python/blob/f36e68549ceca3191e5791e5f99ea1baa1a258b5/src/typesafe_sdk/_core/compression.py),
and [multipart asset implementation](https://github.com/typesafe-ai/typesafe-sdk-js/blob/b844cd1aca12855a5d24a2df52a1401c86e7d37a/src/assets.ts).

TypeSafe documents gzip/x-gzip and zstd request encodings, a 32 MiB limit on
compressed and decompressed request bodies, and an 8 MiB zstd window limit.
Image and long-context opt-in headers must survive forwarding. A smaller capture
budget can stop inspection while still forwarding a valid request. Unsupported
capture formats should produce metadata-only entries rather than fabricated
empty JSON. See [API errors and limits](https://docs.typesafe.ai/api/errors)
and [System One](https://docs.typesafe.ai/api/system-one).

Axum's `Json`, `String`, and `Bytes` extractors default to a 2 MB body limit.
That default is unsuitable as an accidental TypeSafe limit. A streaming handler
needs deliberately chosen transport limits and backpressure. See
[Axum body limits](https://docs.rs/axum/latest/axum/extract/struct.DefaultBodyLimit.html).

Preserve end-to-end headers and statuses, including authentication, experimental
TypeSafe headers, request IDs, rate-limit guidance and content encoding. Rewrite
the upstream authority and remove hop-by-hop headers, including fields named by
`Connection`. Preserve repeated header fields where allowed. Do not promise
identical TCP packets, HTTP framing, header order, or capitalization across a
proxy. See [HTTP connection semantics](https://www.rfc-editor.org/rfc/rfc9110.html#section-7.6.1).

Configure the upstream client deliberately. Reqwest follows redirects by default,
has protocol-level retry behavior, and can perform automatic response
decompression. A faithful diagnostic relay must control those behaviors. Disable
implicit redirects and extra retries; preserve encoded response bytes or correctly
adjust encoding and length if transforming them. Configure corporate proxies
explicitly and prevent an environment-proxy loop. See the
[Reqwest client builder](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html).

Both SDKs default to two retries after the initial attempt. Their retryable
statuses include 408, 429 and 5xx. Python also has a total retry-budget setting;
per-operation and whole-call deadlines are different. The bridge should not
silently add another retry layer. A lost connection after dispatch does not prove
that inference did not happen. An automatic direct fallback could duplicate it.
See the pinned [Python retry policy](https://github.com/typesafe-ai/typesafe-sdk-python/blob/f36e68549ceca3191e5791e5f99ea1baa1a258b5/src/typesafe_sdk/_core/retry.py)
and [JavaScript client](https://github.com/typesafe-ai/typesafe-sdk-js/blob/b844cd1aca12855a5d24a2df52a1401c86e7d37a/src/client.ts).

Cancellation should stop local forwarding and release resources promptly. It
cannot guarantee that TypeSafe stopped already accepted work or did not bill it.
The UI should distinguish client disconnects, upstream failures, local relay
failures, and incomplete responses. An HTTP 200 also does not prove that the
caller's SDK accepted the response schema.

## Data handling and deployment

For a local development product, bind to loopback by default and forward the
application's TypeSafe key without retaining it. Do not record Authorization,
cookies, or credentials in logs, database rows, exported curl commands, or crash
reports. Opt-in payload capture needs redaction before persistence and explicit
retention settings; display masking alone does not remove stored secrets.

Authenticate the inspector and control API, validate Host/Origin where relevant,
and do not enable wildcard browser access. An untrusted web page must not be able
to drive a local gateway or read its history. Keep upstream destinations in trusted
configuration. A request-supplied arbitrary URL would turn the product into an
open proxy or an SSRF route. Replay must obey the same destination controls.

Use bounded memory queues, per-body inspection budgets, image decode limits, disk
quotas and retention. If capture storage is unavailable or falls behind, prioritize
forwarding and report capture loss. Do not silently block inference on a full disk
or a slow UI. This policy is suitable for a debugger; compliance-grade guaranteed
logging would require a different failure policy.

SQLite WAL is a reasonable local starting point with a single writer and short
batched transactions. Keep the database on local disk and account for WAL growth
and checkpoints in quotas. WAL is not a shared database for multiple machines on
a network filesystem. See [SQLite WAL](https://www.sqlite.org/wal.html).

A Docker container's localhost refers to the container. Docker Desktop documents
`host.docker.internal` for reaching the host, but a loopback-only listener may need
a deliberate networking arrangement to be reachable. A sidecar/container service
on the same private network is another option. Do not fix reachability by exposing
an unauthenticated gateway to every interface. See
[Docker Desktop networking](https://docs.docker.com/desktop/features/networking/networking-how-tos/)
and [Compose networking](https://docs.docker.com/compose/how-tos/networking).

A remote backend cannot reach a developer laptop through its own `127.0.0.1`.
For remote use, place the headless daemon near the backend and connect the viewer
through authenticated TLS, a VPN or a tunnel. A shared internal portal also needs
tenant/project isolation, reader and replay permissions, managed credentials and
an operational availability plan. Passing a TypeSafe key over plain LAN HTTP is
not an acceptable extension of the loopback design.

Production traffic through a laptop fails when the laptop sleeps or disconnects.
For production visibility without an additional inference dependency, consider
asynchronous SDK/OpenTelemetry export instead. It requires instrumentation but
keeps the observability service outside the response path.

## Existing tools and the reason to build

LiteLLM already documents TypeSafe pass-through routing under `/typesafe`, model
listing, cost/logging integration, and gateway-owned upstream keys. Its current
documentation also describes a Decisions playground and a unified `/v1/systemone`
route. Therefore, neither a TypeSafe proxy nor a typed decision playground is
unique by itself. These features were verified in documentation, not executed
locally. See [LiteLLM's TypeSafe integration](https://docs.litellm.ai/docs/pass_through/typesafe).

Mitmproxy is an alternative for generic HTTP inspection and reverse-proxy
experiments. Helicone offers a gateway with routing and observability. Langfuse
offers application tracing and evaluation workflows. Their exact suitability for
your desired Jev views needs a product comparison, not an assumption that a
chat-oriented dashboard already renders every Jev type correctly. See
[mitmproxy](https://docs.mitmproxy.org/stable/concepts/modes/),
[Helicone](https://docs.helicone.ai/gateway/overview), and
[Langfuse](https://langfuse.com/docs/observability/overview).

The strongest reason to build is a focused local Jev debugging experience:
question-aware decision views, durable session history, request/answer comparison,
image and box inspection, captured-response fixtures, and links to the application's
Flow execution. Rune can make this a native desktop tool; it does not create the
underlying observability capability.

A smaller alternative is a compatible capture service plus the existing React
viewer. That is the faster path if backend reach and a browser portal are the main
goals. A custom Rust daemon and Rune client are justified if native distribution,
predictable local resource use, and the desktop experience are central goals.
Neither resource use nor speed should be assumed better without measurement.

## A staged experiment worth building next

1. Build the headless Rust relay for the two current endpoints. Preserve bytes and
   statuses, disable added retries, and add bounded capture with explicit fidelity.
   Pass real-SDK contract tests against a local mock before adding UI.
2. Add a Rune request table and detail panel. Include typed answers, request IDs,
   timestamps, errors, filters and visible capture warnings. Keep network work and
   storage independent of the UI. Test close/reopen and GPU startup failure.
3. Add persistent sessions, comparison and export. Keep raw credentials out of
   every artifact. Test quota exhaustion and slow subscribers.
4. Add optional Flow/trace ingestion. Preserve authored definitions and distinguish
   application events from proxy observations. Reuse existing fixtures and
   provenance rules instead of inventing inferred workflows.
5. Add replay or a remote portal only after the capture and authorization contracts
   are stable. A replay that sends live requests is a new billable operation.

Recorded-response fixtures can support offline tests if the capture is complete
and its fidelity is known. Redacted or truncated captures are not exact fixtures.
Replaying a whole Flow can also rerun local tasks with external side effects.
Breakpoints and request editing change timing and may trip SDK deadlines, so they
belong in an explicit debugging mode rather than the default forwarding behavior.

## Validation and performance work still required

The Rust implementation needs protocol tests for compressed JSON, multipart,
headers, repeated headers, body limits, malformed/unknown payloads, all relevant
error statuses, disconnections and cancellation. Include slow uploads, slow
upstreams, slow readers, response truncation, disabled capture, retry counts and
upstream TLS failures. Verify that redirects cannot move credentials to another
host and that the configured proxy cannot route back into itself.

Test storage and UI faults separately: database unavailable, disk full, bounded
queue overflow, capture parsing errors, subscriber reconnection, app termination,
GPU unavailability and long sessions. Include Windows firewall/port conflicts,
Docker and WSL reachability, and real macOS launch, focus, keyboard and rendering
behavior. Packaging, signing and updates are additional delivery work.

Benchmark the same workload in several modes: direct to a controlled mock,
relay with capture disabled, capture enabled in memory, capture persisted, and
capture with the inspector actively rendering. Compare concurrency levels and
small JSON, large JSON, compressed bodies and images. Warm connection pools before
steady-state runs and measure cold startup separately.

Report p50/p95/p99 response latency, throughput, CPU, memory, capture loss, disk
growth and UI input/frame responsiveness. Measure body arrival, upstream response
and transfer completion separately. The relay's elapsed time includes network
and upstream work; it is not pure model inference time. Use controlled mocks to
isolate relay overhead, then a small authorized live test to check real service
compatibility. A mock cannot establish actual TypeSafe latency or cost.

For Windows/macOS comparisons, pin the same app, Rune and SDK revisions, release
profile, fixtures, viewport and scale. Record GPU, driver, refresh rate, power mode
and hardware. Compare relay overhead to each machine's own direct baseline.
Different machines cannot isolate an operating-system effect on their own.
The existing Rune QA runner can supply screenshots and component checks, while a
separate network workload runner measures relay behavior. CPU submission timings
must not be described as displayed frame times.

## Source revisions

- jev-devtools 0.6.0: `b6ccfdd83729235fbfa0ddbea77d89166621fccc`.
- TypeSafe JavaScript SDK 0.7.0: `b844cd1aca12855a5d24a2df52a1401c86e7d37a`.
- TypeSafe Python SDK 0.7.5: `f36e68549ceca3191e5791e5f99ea1baa1a258b5`.
- Rune QA revision: `781e2b6d87cd5f32d6f4365956fad4baaf24ee50`.
- Downloaded JavaScript npm archive SHA-256:
  `27fd552e7df184e13025da3a2a2711133b5307c131f630a6379e950572906cff`.

The experiment used the published SDK packages. Repository commits supplied
source-level evidence; they are not claimed to be reproducible builds of the npm
and PyPI artifacts. Upstream source and public documentation can change after
these checks. Reusing upstream code also requires retaining its license notices
and applicable attribution.
