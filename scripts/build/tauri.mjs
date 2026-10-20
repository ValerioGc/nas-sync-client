import { spawn } from 'node:child_process';
import { fetchEngine } from './engine.mjs';

const [command = 'dev', ...args] = process.argv.slice(2);
const cli =
  process.platform === 'win32' ? '.\\node_modules\\.bin\\tauri.cmd' : 'node_modules/.bin/tauri';

// Builds and development runs carry the sync engine, while the other commands need nothing.
const configCommands = new Set(['build', 'dev']);
let commandArgs = [command, ...args];
if (configCommands.has(command)) {
  try {
    await fetchEngine();
  } catch (error) {
    console.error(`\n❌ Unable to prepare the sync engine: ${error.message}`);
    process.exit(1);
  }
  commandArgs = [
    command,
    '--config',
    'backend/tauri.conf.json',
    '--config',
    'backend/tauri.engine.json',
    ...args,
  ];
}

const child = spawn(cli, commandArgs, {
  shell: process.platform === 'win32',
  stdio: 'inherit',
});

child.on('exit', (code) => process.exit(code ?? 1));
child.on('error', (error) => {
  console.error(`\n❌ Unable to start Tauri: ${error.message}`);
  process.exit(1);
});
