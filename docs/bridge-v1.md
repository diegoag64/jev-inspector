# First-version contract and validation

The first version provides a local Rust TypeSafe relay and a native Rune inspector.
The inspector supports explicit session-file export and import. It has no persistent
session database, replay, Flow-event ingestion, remote access,
automatic upstream fallback, or automatic inference retries.

## Processes and routes

`jev-bridge` serves `POST /v1/systemone` and `GET /v1/models` on a loopback listener,
port 8787 by default. It forwards to one configured upstream origin. HTTPS is
required except for loopback HTTP mocks. Paths, query strings, statuses and
end-to-end headers are forwarded; query strings and raw headers are not retained.
Hop-by-hop headers are removed. Redirects are returned without following them.
System and environment HTTP proxies are disabled to prevent accidental proxy loops.
Corporate proxy configuration is not part of version 1.

The inspection API uses a separate loopback listener, default port 8788:

- `GET /api/requests` returns versioned summaries, a server-instance ID, revision,
  limits and counters.
- `GET /api/requests/{id}` returns a retained exchange and its inspection copies.

Both routes require a random bearer token from the connection file and return
`Cache-Control: no-store`. Unrecognized Host values and requests with browser
Origin or Sec-Fetch-Site headers are rejected. No CORS access is enabled.

The Rune inspector polls summaries on a background thread every 500 ms and fetches
the selected detail when the revision or selection changes. Its mailbox retains
only the newest update. Closing the inspector does not affect a separately started
relay. The convenience demo owns and stops all processes that it starts.

## Capture and forwarding limits

| Setting | Version 1 behavior |
| --- | --- |
| History | Default 100 exchanges, configurable from 1 to 200; oldest entries are evicted |
| Active exchanges | At most 64; additional calls receive a local 503 |
| Request wire limit | 32 MiB; known oversized lengths are rejected before forwarding |
| Capture | Off by default; `--capture-bodies` enables inspection copies |
| Wire inspection budget | First 64 KiB per request and response |
| Decoded inspection budget | 128 KiB per body; zstd window limited to 8 MiB |
| Parsed inspection budget | 4,096 JSON nodes per body |
| Decoder queue | 32 completed exchanges; overflow loses inspection data, not forwarded bytes |
| Upstream deadline | Default 120 seconds, configurable from 1 to 600 seconds |
| Upstream connect deadline | 10 seconds |
| UI JSON preview | First 16,000 UTF-8 bytes, cut at a character boundary; the UI marks this display limit outside the code |

Request and response bodies stream through the relay. Parsing and decompression
run in a separate capture worker. A small capture budget does not truncate the
body being forwarded. The 32 MiB request limit is a transport policy, separate from
capture limits. For a chunked upload that exceeds it, earlier bytes may already
have reached the upstream before the relay rejects the rest.

The inspector decodes JSON, gzip, x-gzip, and zstd. For multipart requests it parses
the JSON `request` part and omits binary image previews. Inline image data under
`assets.*.data` is removed from the inspection copy. Invalid, oversized, incomplete
or unsupported bodies have explicit notes instead of fabricated JSON.

Capture redacts these normalized JSON field names recursively: authorization,
cookie, set-cookie, password, passwd, secret, api-key, access-token, refresh-token,
and token. Normalization removes punctuation and ignores case. This is best-effort
field redaction. It does not identify secrets embedded in prose, unknown field
names or arbitrary response content. Body capture therefore remains opt-in.
Authorization headers are used for forwarding but are not included in the capture
schema or logs. No payloads are persisted by the daemon.

Each history row represents one HTTP exchange. SDK retries remain separate rows.
HTTP 4xx and 5xx responses can have a complete transfer. `Complete` means that the
upstream body was read and produced to the local HTTP server, not that the caller
acknowledged receipt or its SDK accepted the response schema. Known Content-Length
completion is handled without depending on another stream poll.

A disconnect or transport error does not prove that upstream inference stopped.
Observed request bytes are bytes read from the caller, which can include a final
chunk rejected by the transport limit. Elapsed time is relay-observed time, not
pure inference time. The current relay supports the documented HTTP body protocol;
upgrades, WebSockets and forwarding HTTP trailers are not implemented.

## Validation on this Windows machine

The local Rust suite and Clippy checks pass. Protocol tests cover:

- Exact compressed request and response body preservation, authentication and
  experimental headers, request IDs, retry headers, and one upstream attempt.
- Capture truncation without transport truncation and oldest-first eviction.
- Empty HTTP 204 responses completing without a Content-Length header.
- Inspection authentication, Host and Origin checks, and fixed relay routes.
- Redirect responses without following their destination.
- Early delivery of a response chunk before upstream completion, followed by
  recording the caller's disconnect.
- Sixteen concurrent requests completing without an inspector subscriber.
- Rejection of non-loopback listeners, insecure remote upstreams, credentials in
  upstream URLs and direct listener loops.
- Bounded zstd decoding and best-effort field redaction.
- Rune tab and error-filter interactions and search/selection state.

The official JavaScript SDK 0.7.0 and Python SDK 0.7.5 also completed 17 synthetic
scenarios through the actual Rust executable, producing 23 HTTP exchanges. The
test-side observer and mock upstream recorded matching request/response body
hashes. The inspection API retained all 23 attempts, decoded compressed Python
JSON and both multipart image requests, and contained no synthetic API key.
Evidence is in `artifacts/research/rust-bridge-probe.json`.

The JavaScript SDK ran in the tool's Node environment, where normal Node process
detection is unavailable. Its automatic zstd path was disabled. Python zstd was
exercised. Normal Node and macOS probe runs remain unverified in this session.
Run these probes locally; no CI workflow is configured.

The native inspector initialized the NVIDIA GeForce RTX 4080 SUPER through DX12.
The initial sandbox launches created windows the user could not see. Launching
directly outside the sandbox resolved this, and the user confirmed the visible window.
Headless screenshots confirmed the static fixture and live traffic feed using the
same Rune view. The Windows Computer Use helper repeatedly returned
`Computer Use native pipe is unavailable`, including after its JavaScript session
was reset. Real OS mouse/keyboard automation therefore remains unverified. Rune's
own input tests are not a substitute for that platform check.

The tests make no live TypeSafe calls. Upstream TLS against TypeSafe, real billing,
macOS native execution, Linux execution, long-duration load, OS sleep/resume,
installers, signing and production availability have not been validated. This is
a local development tool, with a reproducible mock path for testing those future
changes.
