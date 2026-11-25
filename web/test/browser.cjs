// End-to-end test in Chromium: node test/browser.cjs URL OUTDIR HLADB_DIR FILE... (BAM+BAI or CRAM+CRAI+FA+FAI)
// With HLADB_DIR "-" the page builds the reference itself (downloads hla.dat from GitHub).
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const [url, out, hladb, ...files] = process.argv.slice(2);
  const browser = await chromium.launch({ proxy: process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY, bypass: 'localhost,127.0.0.1' } : undefined });
  const ctx = await browser.newContext({ ignoreHTTPSErrors: true, locale: 'de-DE' });
  const page = await ctx.newPage();
  page.on('console', (m) => console.log('console:', m.text()));
  page.on('pageerror', (e) => console.log('pageerror:', e.message));
  await page.goto(url);
  await page.waitForFunction(() => document.getElementById('ver').textContent !== '');
  if (hladb !== '-') {
    await page.setInputFiles('#ref-dir', hladb);
    await page.waitForFunction(() => document.getElementById('ref-status').classList.contains('ok'));
  }
  console.log('ref:', await page.textContent('#ref-status'));
  await page.setInputFiles('#files', files);
  console.log('files:', (await page.textContent('#file-list')).slice(0, 200), '|', await page.textContent('#file-msg'));
  const t0 = Date.now();
  await page.click('#start');
  let last = '';
  for (;;) {
    await page.waitForTimeout(5000);
    const ph = await page.textContent('#phase');
    if (ph !== last) { console.log(`${((Date.now() - t0) / 1000).toFixed(0)}s`, ph); last = ph; }
    if (!(await page.isHidden('#step-out'))) break;
    if (/Abgebrochen|Stopped/.test(ph)) { console.log(await page.textContent('#log')); process.exit(1); }
  }
  console.log(`done in ${((Date.now() - t0) / 1000).toFixed(0)} s`);
  fs.mkdirSync(out, { recursive: true });
  const links = await page.$$eval('#downloads a', (as) => as.map((a) => [a.download, a.href]));
  for (const [name, href] of links) {
    const b64 = await page.evaluate(async (h) => { const b = new Uint8Array(await (await fetch(h)).arrayBuffer()); let s = ''; for (let i = 0; i < b.length; i += 32768) s += String.fromCharCode(...b.subarray(i, i + 32768)); return btoa(s); }, href);
    fs.writeFileSync(path.join(out, name), Buffer.from(b64, 'base64'));
  }
  await page.setViewportSize({ width: 1200, height: 900 });
  await page.screenshot({ path: path.join(out, 'page.png'), fullPage: false });
  console.log('saved', links.map((l) => l[0]).join(' '));
  await browser.close();
})();
