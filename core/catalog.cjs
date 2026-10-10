'use strict';
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const digest = value => crypto.createHash('sha256').update(value).digest('hex');
const normalize = name => String(name).toLowerCase().replace(/\s*\((?:x64|x86|64-bit|32-bit)\)/g, '').replace(/\s+\d+(?:\.\d+)+(?:\s.*)?$/, '').replace(/\s+/g, ' ').trim();
function reconcile(inventory, wingetRows) {
  const apps = inventory.map(a => ({ ...a, uid: digest(a.key).slice(0, 24), wingetId: null, available: '', evidence: 'Windows inventory' }));
  for (const row of wingetRows) {
    const matches = apps.filter(a => a.kind === 'registry' && normalize(a.name) === normalize(row.name) && a.version === row.version);
    if (matches.length === 1 && !matches[0].wingetId) {
      Object.assign(matches[0], { uid: digest('winget:' + row.id).slice(0, 24), wingetId: row.id, available: row.available, evidence: 'WinGet correlation: unique name and version' });
    } else {
      apps.push({ uid: digest('winget:' + row.id).slice(0, 24), key: 'winget:' + row.id, name: row.name, version: row.version, publisher: '', kind: 'winget', scope: 'reported by WinGet', protected: false, wingetId: row.id, available: row.available, evidence: 'WinGet catalogue entry' });
    }
  }
  // WinGet can report several installed versions with the same ID. Those must not be batch targets.
  const counts = new Map();
  for (const a of apps) if (a.wingetId) counts.set(a.wingetId, (counts.get(a.wingetId) || 0) + 1);
  return apps.map(a => ({ ...a, ambiguous: !!a.wingetId && counts.get(a.wingetId) > 1 }));
}
class Catalog {
  constructor(directory) { this.directory = directory; this.file = path.join(directory, 'catalog.json'); this.data = { schema: 1, scans: 0, apps: {}, receipts: [], logSignals: [], settings: { automatic: false, lastAutoScan: null } }; }
  async load() {
    await fs.mkdir(this.directory, { recursive: true });
    try {
      const data = JSON.parse(await fs.readFile(this.file, 'utf8'));
      if (data.schema !== 1 || !data.apps || !Array.isArray(data.receipts)) throw new Error('Unsupported catalogue format.');
      this.data = data;
      // Interrupted operations are uncertain, never retried automatically.
      for (const receipt of data.receipts) if (receipt.status === 'running') { receipt.status = 'unverified'; receipt.detail = 'AppTide stopped before verification. Scan and review before retrying.'; }
    } catch (e) { if (e.code !== 'ENOENT') throw new Error('The local catalogue cannot be read. Preserve it and restore a known-good backup. ' + e.message); }
    return this;
  }
  async save() {
    // Caller serializes mutations. Keep the previous valid document before atomic replacement.
    const temp = this.file + '.tmp';
    await fs.writeFile(temp, JSON.stringify(this.data, null, 2), { mode: 0o600 });
    try { await fs.copyFile(this.file, this.file + '.bak'); } catch (e) { if (e.code !== 'ENOENT') throw e; }
    await fs.rename(temp, this.file);
  }
  observe(apps, now = new Date().toISOString()) {
    const previous = this.data.apps;
    for (const app of Object.values(previous)) app.present = false;
    for (const observation of apps) {
      const old = previous[observation.uid];
      const versions = old?.versions || [];
      if (observation.version && !versions.some(v => v.version === observation.version)) versions.push({ version: observation.version, seenAt: now });
      previous[observation.uid] = { ...old, ...observation, present: true, firstSeen: old?.firstSeen || now, lastSeen: now, observations: (old?.observations || 0) + 1, versions: versions.slice(-50), excluded: old?.excluded || false, autoUpdate: old?.autoUpdate || false };
    }
    this.data.scans++; this.data.lastScan = now;
  }
}
function classifyReceipt(kind, before, after, exitCode, scanOk) {
  if (!scanOk) return 'unverified';
  if (kind === 'uninstall' && !after) return 'verified';
  if (kind === 'update' && after && before.available === after.version && after.version !== before.version) return 'verified';
  return exitCode === 0 ? 'unverified' : 'failed';
}
module.exports = { Catalog, reconcile, normalize, digest, classifyReceipt };
