'use strict';
const { app, BrowserWindow, ipcMain, dialog, shell } = require('electron');
const path = require('node:path');
const fs = require('node:fs/promises');
const { pathToFileURL } = require('node:url');
const { Service } = require('../core/service.cjs');
const { createDemo } = require('./demo.cjs');
const demo = process.argv.includes('--demo');
const arg = flag => { const i = process.argv.indexOf(flag); return i >= 0 ? process.argv[i + 1] : null; };
const smokeDirectory = demo ? arg('--smoke-dir') : null;
if (smokeDirectory) {
  // Tests must not hand their launch to an already-running real AppTide window.
  app.setPath('userData', path.join(path.resolve(smokeDirectory), 'electron-profile'));
  app.disableHardwareAcceleration();
}
const page = pathToFileURL(path.join(__dirname, '../ui/index.html')).href;
let window, service, timer;
if (!app.requestSingleInstanceLock({ demo })) app.quit();
else {
  app.on('second-instance', () => { if (window) { if (window.isMinimized()) window.restore(); window.focus(); } });
  app.whenReady().then(async () => {
    const directory = arg('--data-dir') || path.join(process.env.LOCALAPPDATA || app.getPath('appData'), demo ? 'AppTide-Demo' : 'AppTide');
    service = await (demo ? createDemo(directory) : new Service(directory)).init();
    if (process.argv.includes('--scan-only')) {
      await service.scan(); const snapshot = service.state();
      console.log(JSON.stringify({ installedRecords: snapshot.apps.filter(a => a.present).length, wingetMatched: snapshot.apps.filter(a => a.present && a.wingetId).length, scans: snapshot.scans, logSignals: snapshot.logSignalCount, warnings: snapshot.warnings }));
      app.exit(0); return;
    }
    const state = () => ({ ...service.state(), demo, version: app.getVersion() });
    window = new BrowserWindow({ width: 1320, height: 860, minWidth: 820, minHeight: 600, title: 'AppTide', backgroundColor: '#f7f9fb', autoHideMenuBar: true, show: !smokeDirectory, webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true, sandbox: true, nodeIntegration: false, webSecurity: true } });
    window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
    window.webContents.on('will-navigate', (e, url) => { if (url !== page) e.preventDefault(); });
    window.webContents.session.setPermissionRequestHandler((_wc, _permission, callback) => callback(false));
    const handle = (name, fn) => ipcMain.handle('apptide:' + name, async (event, ...args) => {
      if (event.sender !== window.webContents || event.senderFrame !== window.webContents.mainFrame || event.senderFrame.url !== page) throw new Error('Untrusted request.');
      try { return { ok: true, value: await fn(...args) }; } catch (e) { return { ok: false, error: e.message }; }
    });
    handle('state', state); handle('scan', () => service.scan());
    handle('prepare', (kind, uids) => service.prepare(kind, uids));
    handle('execute', async token => {
      const plan = service.plans.get(token); if (!plan) throw new Error('Preview expired.');
      if (!demo) {
        const removal = plan.kind === 'uninstall';
        const result = await dialog.showMessageBox(window, { type: removal ? 'warning' : 'question', title: removal ? 'Remove software' : 'Update software', message: removal ? `Uninstall ${plan.apps[0].name}?` : `Update ${plan.apps.length} application${plan.apps.length === 1 ? '' : 's'}?`, detail: plan.apps.map(a => `${a.name} (${a.wingetId})${removal ? '' : ': ' + a.version + ' → ' + a.available}`).join('\n') + '\n\nWindows may request administrator permission. AppTide will verify the result afterward.', buttons: ['Cancel', removal ? 'Uninstall' : 'Update'], defaultId: 0, cancelId: 0 });
        if (result.response !== 1) return { cancelled: true };
      }
      // Return immediately; progress and final results arrive as state events.
      service.execute(token).catch(e => { service.warnings = [e.message]; service.update(); });
      return { started: true };
    });
    handle('preference', (...args) => service.setPreference(...args));
    handle('automatic', async value => {
      if (value && !demo) {
        const answer = await dialog.showMessageBox(window, { type: 'question', message: 'Enable automatic updates for the apps you select?', detail: 'AppTide checks once per day while its window is open. Only apps marked “Allow automatic updates” are updated. No startup task or background service is installed. Installer agreements are not automatically accepted.', buttons: ['Cancel', 'Enable'], defaultId: 0, cancelId: 0 });
        if (answer.response !== 1) return;
      }
      return service.setAutomatic(value);
    });
    handle('portable', async () => { const result = await dialog.showOpenDialog(window, { properties: ['openDirectory'], title: 'Choose a folder containing portable apps' }); if (!result.canceled) return service.addPortableFolder(result.filePaths[0]); });
    handle('watch', (uid, repository) => { if (demo) throw new Error('Release sources are disabled in the synthetic demo.'); return service.watchRepository(uid, repository); });
    handle('release', uid => { const watch = service.catalog.data.apps[uid]?.watch; if (watch) return shell.openExternal(`https://github.com/${watch.repository}/releases`); });
    handle('settings', () => shell.openExternal('ms-settings:appsfeatures'));
    handle('data', () => shell.openPath(service.catalog.directory));
    handle('cancel', () => { service.cancelRequested = true; service.update('Stopping after the current app finishes'); });
    service.on('state', () => { if (!window.isDestroyed()) window.webContents.send('apptide:state', state()); });
    await window.loadURL(page);
    if (demo) await service.scan();
    timer = setInterval(() => service.automaticTick().catch(e => { service.warnings = [e.message]; service.update(); }), 60000);
    window.on('close', event => { if (service.busy) { event.preventDefault(); dialog.showMessageBox(window, { message: 'Wait for the current operation to finish.', detail: 'Use “Stop queue” to skip remaining apps. Closing an active installer would leave the result uncertain.', buttons: ['OK'] }); } });
    if (smokeDirectory) { window.showInactive(); await require('../scripts/smoke-driver.cjs').smoke(window, service, smokeDirectory); }
  }).catch(e => { if (smokeDirectory) { console.error(e); app.exit(1); } else { dialog.showErrorBox('AppTide could not start', e.message); app.quit(); } });
}
app.on('window-all-closed', () => { clearInterval(timer); app.quit(); });
