'use strict';
const path = require('node:path');
(async () => {
  const { packager } = await import('@electron/packager');
  const result = await packager({ dir: path.join(__dirname, '..'), out: process.env.APPTIDE_PACKAGE_OUT || path.join(__dirname, '../dist'), icon: path.join(__dirname, '../assets/apptide.ico'), name: 'AppTide', executableName: 'AppTide', platform: 'win32', arch: 'x64', overwrite: false, asar: { unpack: '**/inventory.ps1' }, prune: true, ignore: [/^\/rust($|\/)/, /^\/test($|\/)/, /^\/artifacts($|\/)/, /^\/dist($|\/)/, /^\/\.git($|\/)/, /^\/\.impeccable($|\/)/, /^\/docs($|\/)/], win32metadata: { CompanyName: 'AppTide contributors', FileDescription: 'AppTide Windows Software Manager', ProductName: 'AppTide' } });
  console.log(result.join('\n'));
})().catch(e => { console.error(e); process.exitCode = 1; });
