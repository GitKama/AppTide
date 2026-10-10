'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { Catalog, reconcile, classifyReceipt } = require('../core/catalog.cjs');
const { parseList, operationArgs } = require('../core/winget.cjs');
const { extractSignals } = require('../core/logs.cjs');
const { Service, eligible, repoPattern } = require('../core/service.cjs');
const { createDemo } = require('../desktop/demo.cjs');
async function temp(t) { const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'apptide-test-')); t.after(() => fs.rm(dir, { recursive: true, force: true })); return dir; }
test('WinGet parsing accepts a localized header, ignores summaries, rejects truncated identifiers', () => {
  const result = parseList('Nombre             Id               Versión   Disponible\n----------------------------------------------------------\n7-Zip              7zip.7zip        24.09     25.01\nTruncated          Vendor.Ap…       1.0       2.0\n2 upgrades available.\n');
  assert.equal(result.rows.length, 1); assert.equal(result.rows[0].available, '25.01'); assert.equal(result.rejected, 2);
});
test('WinGet parse fails closed on a missing table', () => assert.throws(() => parseList('Source agreement is required.')));
test('WinGet identifiers cannot inject shell arguments', () => assert.throws(() => operationArgs('update', { wingetId: 'Foo; shutdown /s', available: '2', version: '1' })));
test('Commands pin exact ID, source, and target version without bypass flags', () => {
  const args = operationArgs('update', { wingetId: 'Vendor.App', available: '2.0', version: '1.0' });
  assert.deepEqual(args, ['upgrade', '--id', 'Vendor.App', '--exact', '--source', 'winget', '--disable-interactivity', '--version', '2.0', '--silent']);
});
test('Unknown installed versions are not upgradable', () => assert.throws(() => operationArgs('update', { wingetId: 'Vendor.App', version: 'Unknown', available: '2.0' })));
test('Unique name and version correlation enriches an inventory record', () => {
  const apps = reconcile([{ key: 'reg:a', name: 'Example 1.0 (x64)', version: '1.0', kind: 'registry' }], [{ name: 'Example', version: '1.0', id: 'Vendor.Example', available: '2.0' }]);
  assert.equal(apps.length, 1); assert.equal(apps[0].wingetId, 'Vendor.Example');
});
test('Ambiguous registry names never merge automatically', () => {
  const apps = reconcile([{ key: 'a', name: 'Example', version: '1', kind: 'registry' }, { key: 'b', name: 'Example', version: '1', kind: 'registry' }], [{ name: 'Example', version: '1', id: 'Vendor.Example', available: '2' }]);
  assert.equal(apps.length, 3); assert.equal(apps.filter(a => a.wingetId).length, 1);
});
test('Multiple installed WinGet versions are flagged as ambiguous', () => {
  const apps = reconcile([], [{ name: 'Example', version: '1', id: 'Vendor.Example', available: '3' }, { name: 'Example', version: '2', id: 'Vendor.Example', available: '3' }]);
  assert.ok(apps.every(a => a.ambiguous));
});
test('Catalogue identity survives a changed uninstall registry key', () => {
  const original = reconcile([{ key: 'old-key', name: 'Example', version: '1', kind: 'registry' }], [{ name: 'Example', version: '1', id: 'Vendor.Example', available: '2' }]);
  const updated = reconcile([{ key: 'new-key', name: 'Example', version: '2', kind: 'registry' }], [{ name: 'Example', version: '2', id: 'Vendor.Example', available: '' }]);
  assert.equal(original[0].uid, updated[0].uid);
});
test('Success requires observed target version, not only exit zero', () => {
  const before = { version: '1', available: '2' };
  assert.equal(classifyReceipt('update', before, { version: '1' }, 0, true), 'unverified');
  assert.equal(classifyReceipt('update', before, { version: '2' }, 0, true), 'verified');
  assert.equal(classifyReceipt('update', before, { version: '2' }, 0, false), 'unverified');
  assert.equal(classifyReceipt('update', before, undefined, 1, true), 'failed');
});
test('Uninstall verification requires a successful fresh scan and absence', () => {
  assert.equal(classifyReceipt('uninstall', {}, undefined, 0, true), 'verified');
  assert.equal(classifyReceipt('uninstall', {}, {}, 0, true), 'unverified');
  assert.equal(classifyReceipt('uninstall', {}, undefined, 0, false), 'unverified');
});
test('Catalogue persists history, preferences, and absent records', async t => {
  const dir = await temp(t); const catalog = await new Catalog(dir).load();
  const a = { uid: 'a', name: 'Example', version: '1' }; catalog.observe([a]); catalog.data.apps.a.excluded = true;
  catalog.observe([{ ...a, version: '2' }]); await catalog.save(); catalog.observe([]); await catalog.save();
  const loaded = await new Catalog(dir).load(); assert.equal(loaded.data.apps.a.versions.length, 2); assert.equal(loaded.data.apps.a.present, false); assert.equal(loaded.data.apps.a.excluded, true); assert.ok(await fs.stat(path.join(dir, 'catalog.json.bak')));
});
test('Corrupt catalogues are preserved instead of silently reset', async t => {
  const dir = await temp(t); await fs.writeFile(path.join(dir, 'catalog.json'), '{ broken');
  await assert.rejects(() => new Catalog(dir).load(), /cannot be read/); assert.equal(await fs.readFile(path.join(dir, 'catalog.json'), 'utf8'), '{ broken');
});
test('Interrupted receipts become unverified on restart', async t => {
  const dir = await temp(t); const c = await new Catalog(dir).load(); c.data.receipts.push({ status: 'running' }); await c.save();
  assert.equal((await new Catalog(dir).load()).data.receipts[0].status, 'unverified');
});
test('Log learning extracts known ID sightings, never commands or substring matches', () => {
  const signals = extractSignals('Found Vendor.App\nrun evil.exe\nFound Other.Vendor.App2', ['Vendor.App', 'Other.Vendor']);
  assert.deepEqual(signals, [{ packageId: 'Vendor.App', type: 'mentioned-in-winget-log' }]);
});
test('GitHub release sources cannot inject a URL or path traversal', () => {
  for (const value of ['https://evil.test/a', 'a/../secret', 'a/b?token=secret', 'a/b#x']) assert.equal(repoPattern.test(value), false);
  assert.equal(repoPattern.test('microsoft/PowerToys'), true);
});
test('Demo update goes through preflight, execution, rescan, and verified receipt', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan();
  const app = service.state().apps.find(eligible); const plan = service.prepare('update', [app.uid]); await service.execute(plan.token);
  assert.equal(service.state().receipts[0].status, 'verified'); assert.equal(service.state().receipts[0].after, app.available);
  await assert.rejects(() => service.execute(plan.token), /expired/);
});
test('Demo uninstall verifies absence without touching installed software', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan(); const a = service.state().apps.find(eligible);
  await service.execute(service.prepare('uninstall', [a.uid]).token); assert.equal(service.state().receipts[0].status, 'verified'); assert.equal(service.state().apps.find(x => x.uid === a.uid).present, false);
});
test('Excluded and protected applications cannot enter update plans', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan(); const a = service.state().apps.find(eligible);
  await service.setPreference(a.uid, 'excluded', true); assert.throws(() => service.prepare('update', [a.uid]), /not eligible/);
  service.catalog.data.apps[a.uid].protected = true; assert.throws(() => service.prepare('uninstall', [a.uid]), /not eligible/);
});
test('Stale preflight stops before executing an installer', async t => {
  let installed = '1', executions = 0;
  const service = await new Service(await temp(t), { inventoryReader: async () => ({ apps: [], warnings: [] }), listReader: async () => ({ rows: [{ name: 'Example', id: 'Vendor.App', version: installed, available: '3' }], rejected: 0 }), logReader: async () => [], runner: async () => { executions++; return { code: 0, stdout: '', stderr: '' }; } }).init();
  await service.scan(); const plan = service.prepare('update', [service.state().apps[0].uid]); installed = '2';
  await assert.rejects(() => service.execute(plan.token), /changed since/); assert.equal(executions, 0);
});
test('An incomplete Windows inventory preserves the previous snapshot', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan(); const count = service.state().apps.length;
  service.inventoryReader = async () => ({ apps: [], warnings: ['AppX failed'] }); await assert.rejects(() => service.scan(), /preserved/);
  assert.equal(service.state().apps.length, count); assert.equal(service.state().busy, false);
});
test('Automatic mode only updates explicitly allowed apps and respects exclusions', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan(); const apps = service.state().apps.filter(eligible);
  await service.setPreference(apps[0].uid, 'autoUpdate', true); await service.setPreference(apps[1].uid, 'autoUpdate', true); await service.setPreference(apps[1].uid, 'excluded', true);
  await service.setAutomatic(true); await service.automaticTick(); assert.equal(service.state().receipts.length, 1); assert.equal(service.state().receipts[0].packageId, apps[0].wingetId);
  await service.automaticTick(); assert.equal(service.state().receipts.length, 1);
});
test('Preference requests cannot modify inherited prototype properties', async t => {
  const service = await createDemo(await temp(t)).init();
  await assert.rejects(() => service.setPreference('__proto__', 'autoUpdate', true), /Invalid/);
  assert.equal({}.autoUpdate, undefined);
});
test('Concurrent catalogue mutations are rejected rather than racing atomic writes', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan(); const a = service.state().apps[0];
  const first = service.setPreference(a.uid, 'excluded', true);
  await assert.rejects(() => service.setPreference(a.uid, 'autoUpdate', true), /already running/);
  await first; assert.equal(service.state().busy, false);
});
test('Unverified automatic updates are not retried on a later day', async t => {
  const service = await createDemo(await temp(t)).init(); await service.scan(); const a = service.state().apps.find(eligible);
  await service.setPreference(a.uid, 'autoUpdate', true); await service.setAutomatic(true);
  service.runner = async () => ({ code: 0, stdout: 'Installer reports success but changes nothing', stderr: '' });
  await service.automaticTick(); assert.equal(service.state().receipts[0].status, 'unverified');
  service.catalog.data.settings.lastAutoScan = null; await service.automaticTick(); assert.equal(service.state().receipts.length, 1);
});
