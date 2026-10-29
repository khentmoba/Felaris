const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const root = path.resolve(__dirname, '..');
const executable = process.env.VELUNO_EXE || [
  process.env.CARGO_TARGET_DIR && path.join(process.env.CARGO_TARGET_DIR, 'debug', 'veluno.exe'),
  path.join(root, 'target/debug/veluno.exe'),
  path.join(root, 'target/release/veluno.exe'),
].filter(Boolean).find(file => fs.existsSync(file));
if (!executable) throw Error('Build with cargo build first, or set VELUNO_EXE.');
const result = spawnSync('python', [path.join(__dirname, 'native_regressions.py'), '--exe', path.resolve(executable)], {
  cwd: root, stdio: 'inherit', windowsHide: true,
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
