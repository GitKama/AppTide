'use strict';
const { Service } = require('../core/service.cjs');
function createDemo(directory) {
  let rows = [
    { id: '7zip.7zip', name: '7-Zip', version: '24.09', available: '25.01' },
    { id: 'VideoLAN.VLC', name: 'VLC media player', version: '3.0.20', available: '3.0.21' },
    { id: 'Microsoft.VisualStudioCode', name: 'Visual Studio Code', version: '1.96.0', available: '1.97.0' },
    { id: 'Mozilla.Firefox', name: 'Mozilla Firefox', version: '135.0', available: '' },
    { id: 'Git.Git', name: 'Git', version: '2.48.1', available: '' }
  ];
  return new Service(directory, {
    inventoryReader: async () => ({ apps: [...rows.map(r => ({ key: 'registry:demo:' + r.id, name: r.name, version: r.version, publisher: r.id.split('.')[0], kind: 'registry', scope: 'user', protected: false })), { key: 'portable:demo', name: 'Field Notes', version: '1.4.0', publisher: 'Example publisher', scope: 'portable', kind: 'portable', protected: false }], warnings: [] }),
    listReader: async () => ({ rows: structuredClone(rows), rejected: 0 }),
    logReader: async () => [{ packageId: '7zip.7zip', hash: 'synthetic-fixture', observedAt: '2026-01-01T00:00:00Z', type: 'mentioned-in-winget-log' }],
    runner: async (_file, args) => { await new Promise(resolve => setTimeout(resolve, 150)); const id = args[args.indexOf('--id') + 1]; if (args[0] === 'upgrade') rows = rows.map(r => r.id === id ? { ...r, version: r.available, available: '' } : r); else rows = rows.filter(r => r.id !== id); return { code: 0, stdout: 'Synthetic test operation. No software changed.', stderr: '' }; }
  });
}
module.exports = { createDemo };
