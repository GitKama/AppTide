'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const directory = path.join(__dirname, '../artifacts/smoke-' + Date.now()); fs.mkdirSync(directory, { recursive: true });
const executable = require('electron');
const environment = { ...process.env }; delete environment.ELECTRON_RUN_AS_NODE;
const result = spawnSync(executable, [path.join(__dirname, '..'), '--demo', '--data-dir', path.join(directory, 'private-data'), '--smoke-dir', directory], { env: environment, encoding: 'utf8', timeout: 60000, windowsHide: true });
console.log(result.stdout || ''); if (result.stderr) console.error(result.stderr); if (result.error) console.error(result.error.message); process.exitCode = result.status ?? 1;
