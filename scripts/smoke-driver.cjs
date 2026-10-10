'use strict';
const { app } = require('electron');
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
async function smoke(window, service, directory) {
  const js = source => window.webContents.executeJavaScript(source);
  const waitFor = async condition => { const deadline = Date.now() + 12000; while (!(await js(condition))) { if (Date.now() > deadline) throw new Error('UI wait timed out: ' + condition); await new Promise(r => setTimeout(r, 70)); } };
  await waitFor("document.querySelectorAll('.app-row').length === 3");
  assert.equal(await js("typeof require"), 'undefined');
  await js("document.querySelector('.app-row').click()");
  assert.ok(await js("document.querySelector('#details h2').textContent.length > 0"));
  await fs.writeFile(path.join(directory, 'desktop.png'), (await window.webContents.capturePage()).toPNG());
  // Reproduce selecting near the end of a real-size inventory, not just three demo rows.
  const originalApps = structuredClone(service.catalog.data.apps);
  const sample = Object.values(originalApps).find(a => a.available);
  for (let i = 0; i < 100; i++) {
    const uid = `scroll-${i}`;
    service.catalog.data.apps[uid] = { ...sample, uid, key: uid, name: `Scroll fixture ${String(i).padStart(3, '0')}`, wingetId: `Demo.Scroll${i}` };
  }
  service.update();
  await waitFor("document.querySelectorAll('.app-row').length === 103");
  await js("document.querySelector('[data-uid=scroll-90]').scrollIntoView({block:'center'});document.querySelector('[data-uid=scroll-90]').click()");
  await new Promise(r => setTimeout(r, 100));
  assert.ok(await js('window.scrollY > 1000'));
  const actionsVisible = "['updateOne','removeOne'].every(id=>{const r=document.getElementById(id).getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight;})";
  assert.ok(await js(actionsVisible), 'Both action buttons stay in view far down the list');
  await fs.writeFile(path.join(directory, 'long-list.png'), (await window.webContents.capturePage()).toPNG());
  await js("document.querySelector('#details').scrollTop=9999;document.querySelector('[data-uid=scroll-91]').click()");
  assert.equal(await js("document.querySelector('#details').scrollTop"), 0);
  assert.ok(await js(actionsVisible), 'Selecting another app restores the panel actions');
  window.setSize(850, 760); await new Promise(r => setTimeout(r, 200));
  await js("document.querySelector('[data-uid=scroll-91]').click()");
  assert.ok(await js(actionsVisible), 'Compact selection brings the stacked actions into view');
  assert.ok(await js('document.documentElement.scrollWidth <= window.innerWidth'));
  await fs.writeFile(path.join(directory, 'compact.png'), (await window.webContents.capturePage()).toPNG());
  service.catalog.data.apps = originalApps; service.update();
  window.setSize(1320, 860);
  await js("window.scrollTo(0,0)");
  await js("document.querySelector('#search').value='no-match';document.querySelector('#search').dispatchEvent(new Event('input'))");
  assert.equal(await js("document.querySelector('#empty').hidden"), false);
  await js("document.querySelector('#search').value='';document.querySelector('#search').dispatchEvent(new Event('input'));document.querySelector('#updateAll').click()");
  await waitFor("document.querySelector('#planDialog').open");
  assert.equal(await js("document.querySelectorAll('.plan-row').length"), 3);
  await js("document.querySelector('#confirmPlan').click()");
  await waitFor("document.querySelector('#updateCount').textContent === '0' && !document.querySelector('#scanButton').disabled");
  assert.equal(service.state().receipts.filter(r => r.status === 'verified').length, 3);
  await js("document.querySelector('[data-view=history]').click()");
  assert.equal(await js("document.querySelectorAll('.receipt').length"), 3);
  await fs.writeFile(path.join(directory, 'history.png'), (await window.webContents.capturePage()).toPNG());
  const receipt = { passed: true, checks: ['isolated renderer', 'updates rendered', 'details selection', 'long inventory keeps actions visible', 'new selection resets details scroll', 'compact selection reveals actions', 'compact layout without overflow', 'search empty state', 'update plan preview', 'three synthetic updates verified', 'history receipts'], synthetic: true };
  await fs.writeFile(path.join(directory, 'result.json'), JSON.stringify(receipt, null, 2));
  console.log(JSON.stringify(receipt)); app.exit(0);
}
module.exports = { smoke };
