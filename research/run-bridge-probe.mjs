import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { runProbe } from './bridge-probe.mjs';

const [sdkPath, pythonPath, relayPath, connectionPath] = process.argv.slice(2);
if (!sdkPath || !pythonPath) {
  console.error('Usage: node research/run-bridge-probe.mjs SDK_MODULE_PATH PYTHON_EXECUTABLE [RUST_RELAY_PATH UNIQUE_CONNECTION_FILE]');
  process.exitCode = 2;
} else {
  try {
    const result = await runProbe({ sdkURL: pathToFileURL(resolve(sdkPath)).href,
      pythonExecutable: resolve(pythonPath), relayExecutable: relayPath ? resolve(relayPath) : undefined,
      connectionFile: connectionPath ? resolve(connectionPath) : undefined });
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(error); process.exitCode = 1;
  }
}
