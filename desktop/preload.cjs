'use strict';
const { contextBridge, ipcRenderer } = require('electron');
const invoke = (method, ...args) => ipcRenderer.invoke('apptide:' + method, ...args);
contextBridge.exposeInMainWorld('appTide', {
  state: () => invoke('state'), scan: () => invoke('scan'),
  prepare: (kind, uids) => invoke('prepare', kind, uids), execute: token => invoke('execute', token),
  preference: (uid, key, value) => invoke('preference', uid, key, value),
  automatic: value => invoke('automatic', value), portable: () => invoke('portable'),
  watch: (uid, repository) => invoke('watch', uid, repository),
  openRelease: uid => invoke('release', uid), openSettings: () => invoke('settings'),
  openData: () => invoke('data'), cancel: () => invoke('cancel'),
  onState: callback => { const listener = (_e, state) => callback(state); ipcRenderer.on('apptide:state', listener); return () => ipcRenderer.removeListener('apptide:state', listener); }
});
