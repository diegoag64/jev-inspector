/**
 * Research only: bounded, buffering HTTP/1 relay between official SDKs and a mock.
 * This is NOT the proposed Rust relay, a streaming implementation, or a benchmark.
 * All listeners bind ephemeral IPv4 loopback ports; all upstream calls are local.
 */
import assert from 'node:assert/strict';
import http from 'node:http';
import { createHash } from 'node:crypto';
import { gzipSync, zstdDecompressSync } from 'node:zlib';
import { execFile, spawn } from 'node:child_process';
import { readFile, stat, unlink } from 'node:fs/promises';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

const exec = promisify(execFile);
const hash = (b) => createHash('sha256').update(b).digest('hex');
const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a3ioAAAAASUVORK5CYII=', 'base64');
async function readBody(stream) {
  const chunks = []; let size = 0;
  for await (const chunk of stream) {
    size += chunk.length;
    assert(size <= 1024 * 1024, 'Probe body exceeds 1 MiB');
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}
function forwardHeaders(headers) {
  const skip = new Set(['host', 'connection', 'keep-alive', 'proxy-authenticate',
    'proxy-authorization', 'te', 'trailer', 'transfer-encoding', 'upgrade',
    ...(headers.connection ?? '').split(',').map(x => x.trim().toLowerCase())]);
  return Object.fromEntries(Object.entries(headers).filter(([key]) => !skip.has(key)));
}
async function listen(server) {
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  return `http://127.0.0.1:${server.address().port}`;
}
function request(url, method, headers, body) {
  return new Promise((resolve, reject) => {
    const req = http.request(url, { method, headers }, async response => {
      try { resolve({ status: response.statusCode, headers: response.headers, body: await readBody(response) }); }
      catch (error) { reject(error); }
    });
    req.on('error', reject);
    req.setTimeout(10000, () => req.destroy(new Error('Probe timeout')));
    req.end(body);
  });
}
async function close(server) {
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
}

export async function runProbe({ sdkURL, pythonExecutable, relayExecutable, connectionFile }) {
  const sdk = await import(sdkURL);
  const attempts = new Map(), upstreamRequests = [], bridgeRequests = [], errors = [];
  function handler(fn) {
    return (req, res) => { void fn(req, res).catch(error => {
      errors.push(String(error)); res.writeHead(500); res.end('Probe failed');
    }); };
  }
  const upstream = http.createServer(handler(async (req, res) => {
    const body = await readBody(req), name = req.headers['x-probe-case'];
    assert.equal(req.headers.authorization, 'Bearer synthetic-probe-key');
    const attempt = (attempts.get(name) ?? 0) + 1;
    attempts.set(name, attempt);
    const decoded = req.headers['content-encoding'] === 'zstd' ? zstdDecompressSync(body) : body;
    if (name.endsWith('compressed')) {
      assert.equal(req.headers['content-encoding'], 'zstd');
      assert(JSON.parse(decoded).state.length > 1024);
    }
    if (name.endsWith('multipart')) {
      assert(req.headers['content-type'].startsWith('multipart/form-data;'));
      assert(decoded.includes(png));
      assert(decoded.includes(Buffer.from('name="request"')));
      assert.equal(req.headers['typesafe-danger-zone-images'], 'true');
    }
    let status = 200;
    if (name.endsWith('auth401')) status = 401;
    if (name.endsWith('retry429')) status = 429;
    if (name.endsWith('retry529') && attempt === 1) status = 529;
    const value = status !== 200 ? { error: { message: 'Synthetic failure' } } :
      req.method === 'GET' ? { models: [{ name: 'probe-model', description: 'Synthetic', release_date: '2026-01-01' }] } :
      { model: 'probe-model', answers: { q: { type: 'noul', noul: 0.75 } }, usage: { input_tokens: 12, output_tokens: 1 } };
    let response = Buffer.from(JSON.stringify(value, null, 2) + '\n');
    const headers = { 'content-type': 'application/json', 'x-typesafe-request-id': 'synthetic-request-id', 'retry-after-ms': '1' };
    if (name.endsWith('response-gzip')) {
      response = gzipSync(response); headers['content-encoding'] = 'gzip';
    }
    headers['content-length'] = String(response.length);
    upstreamRequests.push({ name, attempt, method: req.method, path: req.url,
      requestHash: hash(body), responseHash: hash(response), status,
      contentType: req.headers['content-type'] ?? null, encoding: req.headers['content-encoding'] ?? null });
    res.writeHead(status, headers); res.end(response);
  }));
  const upstreamURL = await listen(upstream);
  let forwardingURL = upstreamURL, relayProcess, connection, startupError;
  const bridge = http.createServer(handler(async (req, res) => {
    assert(['/v1/systemone', '/v1/models'].includes(req.url));
    const body = await readBody(req);
    const result = await request(forwardingURL + req.url, req.method, forwardHeaders(req.headers), body);
    bridgeRequests.push({ name: req.headers['x-probe-case'], requestHash: hash(body),
      responseHash: hash(result.body), status: result.status });
    res.writeHead(result.status, forwardHeaders(result.headers)); res.end(result.body);
  }));
  try {
    if (relayExecutable) {
      assert(connectionFile, 'A unique connectionFile is required for the Rust relay test');
      let exists = false;
      try { await stat(connectionFile); exists = true; }
      catch (error) { if (error.code !== 'ENOENT') throw error; }
      assert(!exists, 'Choose a new connection file; the probe will not replace an existing one');
      relayProcess = spawn(relayExecutable, ['--listen', '127.0.0.1:0', '--inspector', '127.0.0.1:0',
        '--upstream', upstreamURL, '--capture-bodies', '--connection-file', connectionFile], { windowsHide: true, stdio: 'ignore' });
      relayProcess.on('error', error => { startupError = error; });
      const deadline = Date.now() + 10000;
      while (Date.now() < deadline) {
        if (startupError) throw startupError;
        assert(relayProcess.exitCode === null, 'Rust relay exited during startup');
        try {
          connection = JSON.parse(await readFile(connectionFile, 'utf8'));
          const response = await request(connection.inspector_url + '/api/requests', 'GET', {authorization:'Bearer ' + connection.token});
          const snapshot = JSON.parse(response.body);
          if (snapshot.upstream === upstreamURL + '/') { forwardingURL = snapshot.relay_url; break; }
        } catch {}
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      assert.notEqual(forwardingURL, upstreamURL, 'Rust relay startup timed out');
    }
    const baseURL = await listen(bridge);
    const client = new sdk.TypeSafeClient({ apiKey: 'synthetic-probe-key', baseURL });
    const passed = [];
    for (const [name, state, assets] of [
      ['plain', 'synthetic', undefined],
      [client.compression ? 'compressed' : 'large-json', 'synthetic '.repeat(1000), undefined],
      ['multipart', 'Inspect <photo>', { '<photo>': new sdk.Image(png) }],
      ['response-gzip', 'synthetic', undefined],
      ['retry529', 'synthetic', undefined],
    ]) {
      const result = await client.systemOne({ state, questions: { q: { type: 'noul', instructions: 'Is this a test?' } }, assets },
        { headers: { 'x-probe-case': 'js-' + name, 'TypeSafe-Danger-Zone-Images': 'true' } }).withResponse();
      assert.equal(result.data.answers.q.noul, 0.75);
      assert.equal(result.requestId, 'synthetic-request-id');
      passed.push(name);
    }
    const models = await client.models.list({ headers: { 'x-probe-case': 'js-models' } });
    assert.equal(models[0].name, 'probe-model'); passed.push('models');
    for (const [name, ErrorClass] of [['auth401', sdk.AuthenticationError], ['retry429', sdk.RateLimitError]]) {
      await assert.rejects(client.systemOne({ state: 'synthetic', questions: { q: { type: 'noul', instructions: 'Is this a test?' } } },
        { headers: { 'x-probe-case': 'js-' + name } }), error => error instanceof ErrorClass);
      passed.push(name);
    }
    const py = await exec(pythonExecutable, [fileURLToPath(new URL('./bridge-python-client.py', import.meta.url)), baseURL], { timeout: 30000 });
    assert.equal(errors.length, 0, errors.join('\n'));
    assert.equal(bridgeRequests.length, upstreamRequests.length);
    for (let i = 0; i < bridgeRequests.length; i++) {
      const observed = bridgeRequests[i], received = upstreamRequests[i];
      for (const key of ['name', 'requestHash', 'responseHash', 'status']) assert.equal(observed[key], received[key]);
    }
    for (const language of ['js', 'python']) {
      assert.equal(attempts.get(language + '-retry429'), 3);
      assert.equal(attempts.get(language + '-retry529'), 2);
      assert.equal(attempts.get(language + '-auth401'), 1);
    }
    let inspectorChecks;
    if (relayExecutable) {
      const response = await request(connection.inspector_url + '/api/requests', 'GET', {authorization:'Bearer ' + connection.token});
      const snapshot = JSON.parse(response.body);
      assert.equal(snapshot.total_requests, upstreamRequests.length);
      assert.equal(snapshot.capture_dropped, 0);
      assert(snapshot.requests.every(row => row.outcome === 'complete'));
      const details = [];
      for (const row of snapshot.requests) {
        const detail = await request(connection.inspector_url + '/api/requests/' + row.id, 'GET', {authorization:'Bearer ' + connection.token});
        details.push(JSON.parse(detail.body));
      }
      assert(!JSON.stringify(details).includes('synthetic-probe-key'));
      assert(details.some(d => d.request.encoding === 'zstd' && d.request.json?.state));
      assert.equal(details.filter(d => d.request.content_type.startsWith('multipart/') && d.request.json?.assets).length, 2);
      inspectorChecks = 'All HTTP attempts retained; compressed JSON and both image requests decoded; API key absent';
    }
    return { purpose: relayExecutable ? 'Rust relay compatibility with official SDKs and local mocks; not a performance benchmark' : 'Local protocol compatibility, not Rust implementation or performance',
      inspectorChecks,
      javascript: passed, javascriptCompression: client.compression, ...JSON.parse(py.stdout), totalHTTPRequests: upstreamRequests.length,
      bytePreservation: 'SHA-256 matched for every request and response body',
      records: upstreamRequests };
  } catch (error) {
    throw new Error(`${error}\nMock errors: ${errors.join('\n')}`, { cause: error });
  } finally {
    await close(bridge); await close(upstream);
    if (relayProcess?.pid && relayProcess.exitCode === null) {
      const exited = new Promise(resolve => relayProcess.once('exit', resolve));
      relayProcess.kill(); await exited;
    }
    if (connectionFile && connection) { await unlink(connectionFile).catch(() => {}); }
  }
}
