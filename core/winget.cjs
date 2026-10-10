'use strict';
const { run } = require('./process.cjs');
const clean = text => text.replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, '').replace(/\r/g, '');
const validId = id => typeof id === 'string' && /^[A-Za-z0-9][A-Za-z0-9._+-]{1,199}$/.test(id) && !id.includes('...');
function parseList(text) {
  const lines = clean(text).split('\n');
  const divider = lines.findIndex(line => /^\s*-{10,}\s*$/.test(line));
  if (divider < 1) throw new Error('WinGet returned no readable table. No installed apps or updates were inferred.');
  const columns = lines[divider - 1].trim().split(/\s{2,}/);
  if (columns.length < 3 || columns.length > 5) throw new Error('Unsupported WinGet table layout.');
  const rows = []; let rejected = 0;
  for (const line of lines.slice(divider + 1)) {
    if (!line.trim()) continue;
    const cells = line.trim().split(/\s{2,}/);
    if (cells.length < 3 || cells.length > columns.length || !validId(cells[1]) || /[…]|\.\.\./.test(cells[2])) { rejected++; continue; }
    const [name, id, version, fourth = '', fifth = ''] = cells;
    const available = fourth === 'winget' || fourth === 'msstore' ? '' : fourth;
    // Only an explicit catalogue ID from a source-restricted list may be actionable.
    rows.push({ name, id, version, available: /[…]|\.\.\./.test(available) ? '' : available, source: fifth || 'winget' });
  }
  return { rows, rejected };
}
async function list(runner = run) {
  const result = await runner('winget.exe', ['list', '--source', 'winget', '--disable-interactivity']);
  if (result.code !== 0 || result.expired) throw new Error(`WinGet scan failed (${result.code ?? 'timeout'}). Check App Installer and source agreements in Windows Terminal.`);
  return parseList(result.stdout);
}
function operationArgs(kind, app) {
  if (!['update', 'uninstall'].includes(kind) || !validId(app.wingetId)) throw new Error('Invalid package operation.');
  const args = [kind === 'update' ? 'upgrade' : 'uninstall', '--id', app.wingetId, '--exact', '--source', 'winget', '--disable-interactivity'];
  if (kind === 'update') {
    if (!app.available || /^(unknown|<|>)/i.test(app.version) || !app.version) throw new Error('No verified update is available.');
    args.push('--version', app.available, '--silent');
  } else if (app.version && !/^(unknown|<|>)/i.test(app.version)) {
    args.push('--version', app.version);
  }
  // No --force, hash bypass, automatic EULA acceptance, reboot, or purge switches.
  return args;
}
module.exports = { clean, validId, parseList, list, operationArgs };
