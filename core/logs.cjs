'use strict';
const fs = require('node:fs/promises');
const path = require('node:path');
const { digest } = require('./catalog.cjs');
function extractSignals(text, knownIds) {
  const signals = [];
  for (const id of knownIds) {
    // Context is bounded and never interpreted as a command, installer, or trusted source.
    const escaped = id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const match = new RegExp(`(?:^|[^A-Za-z0-9._+-])${escaped}(?=$|[^A-Za-z0-9._+-])`, 'm');
    if (match.test(text)) signals.push({ packageId: id, type: 'mentioned-in-winget-log' });
  }
  return signals;
}
async function readLogSignals(knownIds, localAppData = process.env.LOCALAPPDATA) {
  if (!localAppData) return [];
  const directories = [path.join(localAppData, 'Packages/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe/LocalState/DiagOutputDir'), path.join(localAppData, 'Microsoft/WinGet/DiagOutputDir')];
  const signals = [];
  for (const directory of directories) {
    let names; try { names = await fs.readdir(directory); } catch { continue; }
    const files = await Promise.all(names.filter(n => /^WinGet.*\.log$/i.test(n)).map(async name => ({ name, stat: await fs.stat(path.join(directory, name)).catch(() => null) })));
    for (const file of files.filter(f => f.stat?.isFile() && f.stat.size < 2 * 1024 * 1024).sort((a, b) => b.stat.mtimeMs - a.stat.mtimeMs).slice(0, 30)) {
      const text = await fs.readFile(path.join(directory, file.name), 'utf8').catch(() => '');
      const hash = digest(text);
      for (const signal of extractSignals(text, knownIds)) signals.push({ ...signal, hash, observedAt: file.stat.mtime.toISOString() });
    }
  }
  return signals;
}
module.exports = { extractSignals, readLogSignals };
