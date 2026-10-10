'use strict';
const { spawn } = require('node:child_process');
// Never use a shell. Registry values, log contents, and renderer input cannot become commands.
function run(file, args, { timeout = 120000, onOutput = () => {} } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(file, args, { shell: false, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '', expired = false;
    const limit = 8 * 1024 * 1024;
    const timer = timeout ? setTimeout(() => { expired = true; child.kill(); }, timeout) : null;
    child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
    child.stdout.on('data', chunk => { stdout = (stdout + chunk).slice(-limit); onOutput(chunk); });
    child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-limit); onOutput(chunk); });
    child.on('error', e => { clearTimeout(timer); reject(e); });
    child.on('close', code => { clearTimeout(timer); resolve({ code, stdout, stderr, expired }); });
  });
}
module.exports = { run };
