'use strict';
const $ = id => document.getElementById(id);
const escape = value => String(value ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
let state = { apps: [], receipts: [], settings: {}, scans: 0, logSignalCount: 0 }, view = 'updates', selected = null, plan = null;
const eligible = a => a.present && a.wingetId && a.available && a.version && !/^(unknown|<|>)/i.test(a.version) && !a.excluded && !a.protected && !a.ambiguous;
const date = value => value ? new Date(value).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : 'Not scanned yet';
async function call(method, ...args) { const result = await window.appTide[method](...args); if (!result.ok) throw new Error(result.error); return result.value; }
function notify(text) { $('notice').textContent = text; $('notice').hidden = !text; }
async function act(fn) { try { return await fn(); } catch (e) { notify(e.message); } }
const status = a => !a.present ? ['Previously seen', ''] : a.protected ? ['System component', ''] : a.ambiguous ? ['Review match', 'unmatched'] : a.excluded ? ['Updates paused', ''] : eligible(a) ? ['Update available', 'update'] : a.wingetId ? ['No update reported', ''] : a.watch ? ['Release watch', 'unmatched'] : ['Needs a source', 'unmatched'];
function render() {
  const updates = state.apps.filter(eligible), installed = state.apps.filter(a => a.present && !a.protected);
  $('demoBanner').hidden = !state.demo; $('updateCount').textContent = updates.length; $('installedCount').textContent = installed.length;
  const titles = { updates: ['Updates', 'A little maintenance. Everything in one place.'], installed: ['Installed apps', 'Know what’s here. Decide what stays.'], catalog: ['My catalogue', 'Your software history gets more useful with every scan.'], history: ['Activity', 'What ran, what changed, and what still needs attention.'], settings: ['Settings', 'A software manager that follows your preferences.'] };
  $('heading').textContent = titles[view][0]; $('subtitle').textContent = titles[view][1];
  document.querySelectorAll('[data-view]').forEach(b => { b.classList.toggle('active', b.dataset.view === view); b.setAttribute('aria-current', b.dataset.view === view ? 'page' : 'false'); });
  $('workspace').hidden = ['history', 'settings'].includes(view); $('history').hidden = view !== 'history'; $('settings').hidden = view !== 'settings';
  $('scanButton').disabled = state.busy; $('scanButton').textContent = state.busy ? 'Working…' : 'Check for updates';
  $('updateAll').disabled = state.busy || !updates.length; $('updateAll').hidden = view !== 'updates'; $('updateAll').textContent = updates.length ? `Update all (${updates.length})` : 'Update all';
  $('progress').hidden = !state.busy; $('phase').textContent = state.phase || 'Working'; $('cancelButton').hidden = !state.receipts.some(r => r.status === 'running');
  $('scanSummary').textContent = state.scans ? `${updates.length} updates available · ${installed.length} app records · ${installed.filter(a => !a.wingetId).length} without a WinGet source` : 'Your first scan builds your private software catalogue.';
  $('lastScan').textContent = state.lastScan ? `Checked ${date(state.lastScan)}` : 'Not scanned yet';
  $('catalogSummary').textContent = `Private catalogue · ${state.scans} scans · ${state.apps.length} records · ${state.logSignalCount} log observations`;
  $('automatic').checked = !!state.settings.automatic; $('automatic').disabled = state.busy; $('portable').disabled = state.busy;
  if (state.warnings?.length) notify(state.warnings.join(' '));
  renderList(); renderDetails(); renderHistory();
}
function renderList() {
  const query = $('search').value.toLowerCase();
  let apps = state.apps.filter(a => (view === 'catalog' || a.present) && ($('showSystem').checked || !a.protected));
  if (view === 'updates') apps = apps.filter(eligible);
  apps = apps.filter(a => `${a.name} ${a.publisher} ${a.wingetId || ''}`.toLowerCase().includes(query)).sort((a, b) => a.name.localeCompare(b.name));
  $('appList').innerHTML = apps.map(a => { const [label, style] = status(a); return `<button role="listitem" class="app-row ${selected === a.uid ? 'selected' : ''}" data-uid="${a.uid}" aria-label="${escape(a.name + ', ' + label)}"><span><span class="app-name">${escape(a.name)}</span><span class="app-meta">${escape(a.publisher || a.wingetId || a.kind)}</span></span><span class="version-cell">${escape(a.version || 'Unknown')}${a.available ? `<small>to ${escape(a.available)}</small>` : ''}</span><span class="status ${style}">${label}</span></button>`; }).join('');
  $('empty').hidden = !!apps.length;
  $('emptyTitle').textContent = query ? 'No matching applications' : !state.scans ? 'Let’s see what’s installed' : view === 'updates' ? 'No actionable updates reported' : 'No applications in this view';
  $('emptyText').textContent = query ? 'Try another name or clear the search.' : !state.scans ? 'Check for updates to discover installed apps and begin building your catalogue. The scan doesn’t change your software.' : view === 'updates' ? 'Check Installed apps for excluded apps, missing sources, and software outside WinGet’s coverage.' : 'Try including system components, or run a fresh scan.';
  $('appList').querySelectorAll('[data-uid]').forEach(button => button.addEventListener('click', () => { selected = button.dataset.uid; renderList(); renderDetails(); }));
}
function renderDetails() {
  const a = state.apps.find(a => a.uid === selected); if (!a) return;
  const canRemove = a.present && a.wingetId && !a.protected && !a.ambiguous;
  $('details').innerHTML = `<h2>${escape(a.name)}</h2><p class="publisher">${escape(a.publisher || a.kind)}</p><dl class="facts"><div><dt>Installed version</dt><dd>${escape(a.version || 'Unknown')}</dd></div><div><dt>Update source</dt><dd>${escape(a.wingetId || 'No verified update source')}</dd></div><div><dt>Evidence</dt><dd>${escape(a.evidence)}</dd></div><div><dt>Local knowledge</dt><dd>${a.observations} scans · ${a.logMentions || 0} log observations</dd></div></dl>${eligible(a) ? '<button id="updateOne" class="button primary">Update this app</button>' : ''}${canRemove ? '<button id="removeOne" class="button danger">Uninstall app</button>' : a.present && !a.protected ? '<button id="windowsSettings" class="button secondary">Manage in Windows Settings</button>' : ''}<div class="detail-options"><label><input id="excludeApp" type="checkbox" ${a.excluded ? 'checked' : ''}> Pause updates for this app</label><label><input id="autoApp" type="checkbox" ${a.autoUpdate ? 'checked' : ''} ${!a.wingetId || a.protected || a.ambiguous ? 'disabled' : ''}> Allow automatic updates</label></div><h3>Observed versions</h3><ul class="timeline">${a.versions.slice(-5).reverse().map(v => `<li>${escape(v.version)} · ${escape(new Date(v.seenAt).toLocaleDateString())}</li>`).join('') || '<li>No version reported</li>'}</ul><h3>Watch a GitHub release</h3><p class="help">Track a public project outside WinGet. Release watches never install files automatically.</p><div class="source-form"><input id="repository" placeholder="owner/repository" aria-label="GitHub release repository" value="${escape(a.watch?.repository || '')}"><button id="saveWatch" class="button secondary">${a.watch ? 'Refresh release' : 'Add release watch'}</button></div>${a.watch ? `<p class="help">Latest release reported: ${escape(a.watch.version)}<br>Checked ${escape(date(a.watch.checkedAt))}</p><button id="openRelease" class="text-button">Open releases on GitHub</button>` : ''}`;
  $('updateOne')?.addEventListener('click', () => review('update', [a.uid])); $('removeOne')?.addEventListener('click', () => review('uninstall', [a.uid]));
  $('windowsSettings')?.addEventListener('click', () => act(() => call('openSettings')));
  $('excludeApp').addEventListener('change', e => act(() => call('preference', a.uid, 'excluded', e.target.checked)));
  $('autoApp').addEventListener('change', e => act(() => call('preference', a.uid, 'autoUpdate', e.target.checked)));
  $('saveWatch').addEventListener('click', () => act(() => call('watch', a.uid, $('repository').value.trim())));
  $('openRelease')?.addEventListener('click', () => act(() => call('openRelease', a.uid)));
  if (state.busy) $('details').querySelectorAll('button,input').forEach(el => { el.disabled = true; });
}
function renderHistory() {
  $('history').innerHTML = state.receipts.length ? state.receipts.map(r => `<article class="receipt"><div><h3>${escape(r.name)} <span class="muted">${r.kind === 'update' ? 'Update' : 'Uninstall'}</span></h3><p>${escape(r.before)}${r.kind === 'update' ? ' → ' + escape(r.target) : ''}</p><p>${escape(r.detail || 'Operation in progress')}</p><time>${escape(date(r.startedAt))}</time></div><strong class="${escape(r.status)}">${escape({ verified: 'Verified', failed: 'Failed', unverified: 'Needs review', running: 'Running' }[r.status] || r.status)}</strong></article>`).join('') : '<div class="empty"><h2>A record you can rely on</h2><p>Updates and removals appear here, with a fresh scan to confirm what actually changed.</p></div>';
}
async function review(kind, uids) {
  await act(async () => {
    plan = await call('prepare', kind, uids);
    $('planTitle').textContent = kind === 'update' ? 'Review updates' : 'Review removal';
    $('planDescription').textContent = kind === 'update' ? 'These apps will be updated sequentially from their WinGet sources.' : 'The application’s uninstaller will run. Removing software can affect its settings and data.';
    $('planApps').innerHTML = plan.apps.map(a => `<div class="plan-row"><span>${escape(a.name)}<small>${escape(a.wingetId)}</small></span><span>${escape(a.version)}${kind === 'update' ? ' → ' + escape(a.available) : ''}</span></div>`).join('');
    $('confirmPlan').textContent = kind === 'update' ? 'Update apps' : 'Uninstall app'; $('planDialog').showModal();
  });
}
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => { view = button.dataset.view; $('search').value = ''; render(); }));
$('scanButton').addEventListener('click', () => { notify(''); act(() => call('scan')); });
$('updateAll').addEventListener('click', () => review('update', state.apps.filter(eligible).slice(0, 100).map(a => a.uid)));
$('confirmPlan').addEventListener('click', () => { $('planDialog').close(); if (plan) act(() => call('execute', plan.token)); plan = null; });
$('search').addEventListener('input', renderList); $('showSystem').addEventListener('change', renderList);
$('automatic').addEventListener('change', e => act(async () => { await call('automatic', e.target.checked); state = await call('state'); render(); }));
$('portable').addEventListener('click', () => act(() => call('portable'))); $('openData').addEventListener('click', () => act(() => call('openData')));
$('cancelButton').addEventListener('click', () => act(() => call('cancel')));
window.appTide.onState(next => { state = next; render(); });
act(async () => { state = await call('state'); render(); });
