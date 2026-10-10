'use strict';
const path = require('node:path');
const { Service, eligible } = require('../core/service.cjs');
async function main() {
  if (process.platform !== 'win32') throw new Error('Live scans require Windows. Use npm test for synthetic fixtures on other platforms.');
  const index = process.argv.indexOf('--data-dir');
  const directory = index >= 0 ? path.resolve(process.argv[index + 1]) : path.join(process.env.LOCALAPPDATA, 'AppTide');
  const service = await new Service(directory).init(); await service.scan(); const state = service.state();
  console.log(JSON.stringify({ installedRecords: state.apps.filter(a => a.present).length, wingetMatched: state.apps.filter(a => a.present && a.wingetId).length, updates: state.apps.filter(eligible).length, scans: state.scans, logSignals: state.logSignalCount, warnings: state.warnings }, null, 2));
}
main().catch(e => { console.error(e.message); process.exitCode = 1; });
