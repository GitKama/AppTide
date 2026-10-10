'use strict';
const { EventEmitter } = require('node:events');
const path = require('node:path');
const fs = require('node:fs/promises');
const { randomUUID } = require('node:crypto');
const { run } = require('./process.cjs');
const winget = require('./winget.cjs');
const { Catalog, reconcile, classifyReceipt } = require('./catalog.cjs');
const { readLogSignals } = require('./logs.cjs');
const protectedIds = /^(Microsoft\.(AppInstaller|WindowsAppRuntime.*|WindowsTerminal.*)|AppTide\..*)$/i;
const repoPattern = /^[A-Za-z0-9][A-Za-z0-9-]{0,38}\/[A-Za-z0-9_.-]{1,100}$/;
function eligible(a) { return a.present && a.wingetId && a.available && a.version && !/^(unknown|<|>)/i.test(a.version) && !a.protected && !a.ambiguous && !a.excluded; }
class Service extends EventEmitter {
  constructor(directory, options = {}) {
    super(); this.catalog = new Catalog(directory); this.runner = options.runner || run; this.inventoryReader = options.inventoryReader; this.listReader = options.listReader; this.logReader = options.logReader || readLogSignals;
    this.busy = false; this.phase = ''; this.warnings = []; this.plans = new Map(); this.cancelRequested = false; this.portableFolders = [];
  }
  async init() { await this.catalog.load(); this.portableFolders = this.catalog.data.portableFolders || []; return this; }
  state() {
    return { apps: Object.values(this.catalog.data.apps), receipts: this.catalog.data.receipts.slice(-100).reverse(), scans: this.catalog.data.scans, lastScan: this.catalog.data.lastScan, logSignalCount: this.catalog.data.logSignals.length, settings: this.catalog.data.settings, busy: this.busy, phase: this.phase, warnings: this.warnings, portableFolderCount: this.portableFolders.length };
  }
  update(phase) { if (phase !== undefined) this.phase = phase; this.emit('state', this.state()); }
  assertIdle() { if (this.busy) throw new Error('An operation is already running. Wait for it to finish.'); }
  async inventory(folder) {
    if (this.inventoryReader) return this.inventoryReader(folder);
    const script = path.join(__dirname, '../scripts/inventory.ps1').replace(/app\.asar([\\/])/, 'app.asar.unpacked$1');
    const args = ['-NoProfile', '-NonInteractive', '-File', script];
    if (folder) args.push('-PortableFolder', folder);
    const result = await this.runner(path.join(process.env.SystemRoot || 'C:\\Windows', 'System32/WindowsPowerShell/v1.0/powershell.exe'), args);
    if (result.code !== 0) throw new Error('Windows inventory failed. Check PowerShell script policy and file access; no policy was changed.');
    return JSON.parse(result.stdout.replace(/^\uFEFF/, ''));
  }
  async collect() {
    this.update('Reading installed software');
    const inventory = await this.inventory();
    if (inventory.warnings?.length) throw new Error(inventory.warnings.join(' ') + ' Previous inventory preserved.');
    const warnings = [];
    for (const folder of this.portableFolders) {
      try { const portable = await this.inventory(folder); inventory.apps.push(...portable.apps); }
      catch { warnings.push('A selected portable folder could not be scanned.'); }
    }
    this.update('Checking WinGet sources');
    let result = { rows: [], rejected: 0 }; let wingetOk = true;
    try { result = await (this.listReader ? this.listReader() : winget.list(this.runner)); } catch (e) { warnings.push(e.message); wingetOk = false; }
    if (result.rejected) warnings.push(`${result.rejected} WinGet lines were not actionable (including summaries or unsupported rows).`);
    const apps = reconcile(inventory.apps, result.rows).map(a => ({ ...a, protected: a.protected || protectedIds.test(a.wingetId || '') }));
    this.catalog.observe(apps);
    this.warnings = warnings;
    this.update('Learning from local logs');
    const logSignals = await this.logReader(result.rows.map(r => r.id)).catch(() => []);
    const signals = new Map(this.catalog.data.logSignals.map(s => [s.hash + s.packageId, s]));
    for (const s of logSignals) signals.set(s.hash + s.packageId, s);
    this.catalog.data.logSignals = [...signals.values()].slice(-5000);
    for (const a of Object.values(this.catalog.data.apps)) a.logMentions = this.catalog.data.logSignals.filter(s => s.packageId === a.wingetId).length;
    await this.catalog.save();
    return { apps, wingetOk, inventoryOk: !warnings.some(w => w.includes('portable folder')) };
  }
  async scan() {
    this.assertIdle(); this.busy = true;
    try { await this.collect(); return this.state(); }
    finally { this.busy = false; this.update(''); }
  }
  async setPreference(uid, key, value) {
    this.assertIdle();
    const a = Object.hasOwn(this.catalog.data.apps, uid) ? this.catalog.data.apps[uid] : null;
    if (!a || !['excluded', 'autoUpdate'].includes(key) || typeof value !== 'boolean') throw new Error('Invalid app preference.');
    if (key === 'autoUpdate' && value && (!a.wingetId || a.protected || a.ambiguous)) throw new Error('Only unambiguous WinGet apps can be selected for automatic updates.');
    this.busy = true;
    try { a[key] = value; await this.catalog.save(); } finally { this.busy = false; this.update(); }
  }
  async setAutomatic(value) {
    this.assertIdle(); if (typeof value !== 'boolean') throw new Error('Invalid setting.');
    this.busy = true;
    try { this.catalog.data.settings.automatic = value; await this.catalog.save(); } finally { this.busy = false; this.update(); }
  }
  async addPortableFolder(folder) {
    this.assertIdle();
    this.busy = true;
    try {
      const stat = await fs.stat(folder); if (!stat.isDirectory()) throw new Error('Select a directory.');
      if (!this.portableFolders.includes(folder)) this.portableFolders.push(folder);
      this.catalog.data.portableFolders = this.portableFolders;
      await this.catalog.save();
    } finally { this.busy = false; }
    return this.scan();
  }
  async watchRepository(uid, repository) {
    this.assertIdle(); const app = Object.hasOwn(this.catalog.data.apps, uid) ? this.catalog.data.apps[uid] : null;
    if (!app || typeof repository !== 'string' || !repoPattern.test(repository)) throw new Error('Use a public GitHub repository in owner/repository form.');
    this.busy = true; this.update('Checking the release source');
    try {
      const response = await fetch(`https://api.github.com/repos/${repository}/releases/latest`, { headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'AppTide' }, signal: AbortSignal.timeout(15000), redirect: 'error' });
      if (!response.ok) throw new Error(`GitHub returned ${response.status}. Check the repository name, published releases, and API rate limit.`);
      const release = await response.json();
      app.watch = { repository, version: String(release.tag_name).slice(0, 150), checkedAt: new Date().toISOString(), publishedAt: release.published_at, status: 'watch-only', url: `https://github.com/${repository}/releases` };
      await this.catalog.save(); return app.watch;
    } finally { this.busy = false; this.update(''); }
  }
  prepare(kind, uids) {
    this.assertIdle();
    if (!['update', 'uninstall'].includes(kind) || !Array.isArray(uids) || !uids.length || uids.length > 100 || (kind === 'uninstall' && uids.length !== 1)) throw new Error('Invalid operation request.');
    const apps = [...new Set(uids)].map(uid => Object.hasOwn(this.catalog.data.apps, uid) ? this.catalog.data.apps[uid] : null);
    for (const app of apps) {
      if (!app?.present || !app.wingetId || app.protected || app.ambiguous || (kind === 'update' && !eligible(app))) throw new Error('One or more apps are not eligible. Rescan and review exclusions or source coverage.');
    }
    const token = randomUUID();
    const plan = { token, kind, createdAt: Date.now(), apps: apps.map(a => ({ ...a })) };
    this.plans.clear(); this.plans.set(token, plan);
    return { token, kind, apps: plan.apps.map(a => ({ name: a.name, version: a.version, available: a.available, wingetId: a.wingetId })) };
  }
  async execute(token) {
    this.assertIdle();
    const plan = this.plans.get(token); this.plans.delete(token);
    if (!plan || Date.now() - plan.createdAt > 600000) throw new Error('This preview expired. Review a new preview.');
    this.busy = true; this.cancelRequested = false;
    try {
      const fresh = await this.collect();
      if (!fresh.wingetOk || !fresh.inventoryOk) throw new Error('A complete preflight scan is required before changing software.');
      for (const before of plan.apps) {
        const now = Object.values(this.catalog.data.apps).filter(a => a.present && a.wingetId === before.wingetId);
        if (now.length !== 1 || now[0].version !== before.version || now[0].protected || now[0].ambiguous || (plan.kind === 'update' && (!eligible(now[0]) || now[0].available !== before.available))) throw new Error('Installed software changed since the preview. Review a fresh plan.');
      }
      for (const before of plan.apps) {
        if (this.cancelRequested) break;
        const receipt = { id: randomUUID(), kind: plan.kind, packageId: before.wingetId, name: before.name, before: before.version, target: before.available, startedAt: new Date().toISOString(), status: 'running' };
        this.catalog.data.receipts.push(receipt); this.catalog.data.receipts = this.catalog.data.receipts.slice(-1000);
        await this.catalog.save(); this.update(`${plan.kind === 'update' ? 'Updating' : 'Removing'} ${before.name}`);
        try {
          // Installer operations have no forced timeout. Stop only cancels queued apps.
          const result = await this.runner('winget.exe', winget.operationArgs(plan.kind, before), { timeout: 0 });
          const receiptDir = path.join(this.catalog.directory, 'receipts'); await fs.mkdir(receiptDir, { recursive: true });
          await fs.writeFile(path.join(receiptDir, receipt.id + '.log'), result.stdout + '\n' + result.stderr, { mode: 0o600 });
          receipt.exitCode = result.code;
          let afterScan;
          try { afterScan = await this.collect(); } catch { afterScan = { apps: [], wingetOk: false, inventoryOk: false }; }
          const after = afterScan.apps.find(a => a.wingetId === before.wingetId || a.key === before.key);
          receipt.after = after?.version || null;
          receipt.status = classifyReceipt(plan.kind, before, after, result.code, afterScan.wingetOk && afterScan.inventoryOk);
          receipt.detail = receipt.status === 'verified' ? 'Confirmed by a fresh installed-software scan.' : 'The requested state was not confirmed. Review the local receipt before retrying; elevation, an agreement, or an app restart may be required.';
        } catch (e) { receipt.status = 'unverified'; receipt.detail = e.message; }
        receipt.finishedAt = new Date().toISOString(); await this.catalog.save(); this.update();
        if (receipt.status !== 'verified') break; // Do not cascade failures into the remainder of a batch.
      }
    } finally { this.busy = false; this.update(''); }
    return this.state();
  }
  async automaticTick() {
    if (this.busy || !this.catalog.data.settings.automatic) return;
    const last = Date.parse(this.catalog.data.settings.lastAutoScan || '') || 0;
    if (Date.now() - last < 24 * 60 * 60 * 1000) return;
    this.busy = true;
    try { this.catalog.data.settings.lastAutoScan = new Date().toISOString(); await this.catalog.save(); await this.collect(); }
    finally { this.busy = false; this.update(''); }
    // Any failed/uncertain receipt blocks unattended retries until a later verified manual result.
    const apps = Object.values(this.catalog.data.apps).filter(a => {
      const lastReceipt = this.catalog.data.receipts.findLast(r => r.packageId === a.wingetId);
      return eligible(a) && a.autoUpdate && (!lastReceipt || lastReceipt.status === 'verified');
    });
    if (apps.length) { const plan = this.prepare('update', apps.slice(0, 100).map(a => a.uid)); await this.execute(plan.token); }
  }
}
module.exports = { Service, eligible, repoPattern };
