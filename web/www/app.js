// hla-wgs in the browser: reference setup (IndexedDB cache), file choice, run in a Web Worker,
// result page and downloads. Nothing of the user's data leaves the computer: the only network
// requests are the reference files from IPD-IMGT/HLA (GitHub) and the files of this page.
import { version, db_format } from './pkg/hla_wgs.js';
import initWasm from './pkg/hla_wgs.js';

const RELEASE = '3.66';
const BRANCH = '3660';
const IMGT = `https://raw.githubusercontent.com/ANHIG/IMGTHLA/${BRANCH}`;
const WMDA = ['hla_nom_g.txt', 'hla_nom_p.txt', 'rel_dna_ser.txt', 'rel_ser_ser.txt'];
const $ = (id) => document.getElementById(id);

// ── language ────────────────────────────────────────────────────────────────
let lang = localStorage.getItem('hla-wgs-lang') || (navigator.language || 'de').slice(0, 2);
if (lang !== 'de' && lang !== 'en') lang = 'en';
const setLang = (l) => { lang = l; document.body.dataset.lang = l; document.documentElement.lang = l; localStorage.setItem('hla-wgs-lang', l); refreshTexts(); };
document.querySelectorAll('.seg button').forEach((b) => b.addEventListener('click', () => setLang(b.dataset.set)));
const T = (de, en) => (lang === 'de' ? de : en);
let refreshTexts = () => {};

// ── reference cache (IndexedDB) ─────────────────────────────────────────────
let FORMAT = 0;
const dbKey = () => `hladb-${RELEASE}-f${FORMAT}`;
function idb() {
  return new Promise((res, rej) => {
    const r = indexedDB.open('hla-wgs', 1);
    r.onupgradeneeded = () => r.result.createObjectStore('ref');
    r.onsuccess = () => res(r.result);
    r.onerror = () => rej(r.error);
  });
}
async function idbGet(key) {
  const d = await idb();
  return new Promise((res, rej) => { const q = d.transaction('ref').objectStore('ref').get(key); q.onsuccess = () => res(q.result); q.onerror = () => rej(q.error); });
}
async function idbPut(key, val) {
  const d = await idb();
  return new Promise((res, rej) => { const t = d.transaction('ref', 'readwrite'); t.objectStore('ref').put(val, key); t.oncomplete = () => res(); t.onerror = () => rej(t.error); });
}
async function idbClear() {
  const d = await idb();
  return new Promise((res) => { const t = d.transaction('ref', 'readwrite'); t.objectStore('ref').clear(); t.oncomplete = () => res(); });
}

let reference = null; // {name: Uint8Array}
let refBusy = false;

function showRef() {
  const s = $('ref-status');
  s.className = 'status';
  if (refBusy) return;
  if (reference) {
    const rel = new TextDecoder().decode(reference.release || new Uint8Array()).trim() || RELEASE;
    s.textContent = T(`Bereit: IPD-IMGT/HLA ${rel}, im Browser gespeichert.`, `Ready: IPD-IMGT/HLA ${rel}, stored in the browser.`);
    s.classList.add('ok');
    $('ref-build').hidden = true;
    $('ref-clear').hidden = false;
  } else {
    s.textContent = T('Noch nicht eingerichtet. Das geschieht einmalig, spätestens beim ersten Start.', 'Not set up yet. This happens once, at the latest at the first start.');
    $('ref-build').hidden = false;
    $('ref-clear').hidden = true;
  }
}

async function fetchBytes(url, onProgress) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url}: HTTP ${r.status}`);
  const total = +r.headers.get('content-length') || 0;
  const rd = r.body.getReader();
  const parts = [];
  let n = 0;
  for (;;) {
    const { done, value } = await rd.read();
    if (done) break;
    parts.push(value); n += value.length;
    if (onProgress) onProgress(n, total);
  }
  return new Blob(parts);
}

// the one member of a ZIP file, unpacked by the browser (DecompressionStream)
async function unzipOne(zip, want) {
  const u8 = async (a, b) => new Uint8Array(await zip.slice(a, b).arrayBuffer());
  const tail = await u8(Math.max(0, zip.size - 65558), zip.size);
  const dv = new DataView(tail.buffer);
  let eocd = -1;
  for (let i = tail.length - 22; i >= 0; i--) if (dv.getUint32(i, true) === 0x06054b50) { eocd = i; break; }
  if (eocd < 0) throw new Error('hla.dat.zip: not a ZIP file');
  const cdSize = dv.getUint32(eocd + 12, true), cdOff = dv.getUint32(eocd + 16, true);
  const cd = await u8(cdOff, cdOff + cdSize);
  const cv = new DataView(cd.buffer);
  for (let p = 0; p + 46 <= cd.length;) {
    const method = cv.getUint16(p + 10, true), csize = cv.getUint32(p + 20, true);
    const nlen = cv.getUint16(p + 28, true), xlen = cv.getUint16(p + 30, true), clen = cv.getUint16(p + 32, true);
    const local = cv.getUint32(p + 42, true);
    const name = new TextDecoder().decode(cd.subarray(p + 46, p + 46 + nlen));
    if (name === want) {
      const lh = new DataView((await u8(local, local + 30)).buffer);
      const start = local + 30 + lh.getUint16(26, true) + lh.getUint16(28, true);
      const data = zip.slice(start, start + csize);
      if (method === 0) return data;
      if (method !== 8) throw new Error(`hla.dat.zip: compression method ${method}`);
      return await new Response(data.stream().pipeThrough(new DecompressionStream('deflate-raw'))).blob();
    }
    p += 46 + nlen + xlen + clen;
  }
  throw new Error(`hla.dat.zip: ${want} missing`);
}

function worker() {
  return new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
}

// a job in a fresh worker; resolves with the written files
function job(msg, transfer, onLog) {
  return new Promise((resolve, reject) => {
    const w = worker();
    w.onmessage = (ev) => {
      const m = ev.data;
      if (m.type === 'log') onLog(m.line);
      else if (m.type === 'done') { w.terminate(); resolve(m.files); }
      else if (m.type === 'error') { w.terminate(); reject(new Error(m.message)); }
    };
    w.onerror = (e) => { w.terminate(); reject(new Error(e.message || 'worker error')); };
    w.postMessage(msg, transfer || []);
  });
}

async function buildReference(ui) {
  refBusy = true;
  const s = $('ref-status');
  s.className = 'status';
  $('ref-build').disabled = true;
  try {
    ui.phase(T('Referenz: hla.dat wird geladen …', 'Reference: downloading hla.dat …'), 0);
    const zip = await fetchBytes(`${IMGT}/hla.dat.zip`, (n, t) => ui.phase(T(`Referenz: hla.dat wird geladen, ${(n / 1048576).toFixed(0)} von ${(t / 1048576).toFixed(0)} MB`, `Reference: downloading hla.dat, ${(n / 1048576).toFixed(0)} of ${(t / 1048576).toFixed(0)} MB`), t ? n / t * 0.1 : 0));
    ui.phase(T('Referenz: entpacken …', 'Reference: unpacking …'), 0.1);
    const dat = await unzipOne(zip, 'hla.dat');
    const extra = {};
    for (const f of WMDA) extra[f] = new Uint8Array(await (await fetchBytes(`${IMGT}/wmda/${f}`)).arrayBuffer());
    extra['panel.fa.mask.bed'] = new Uint8Array(await (await fetchBytes(new URL('./data/panel_mask_3.66.bed', import.meta.url))).arrayBuffer());
    const freq = await (await fetchBytes(new URL('./data/freq_1000g_2field.json', import.meta.url))).arrayBuffer();
    ui.phase(T('Referenz wird gebaut (einmalig, etwa 5 bis 10 Minuten) …', 'Building the reference (once, about 5 to 10 minutes) …'), 0.12, true);
    const files = await job({ cmd: 'build-db', dat, freq }, [freq], (line) => ui.log(line));
    const ref = { ...files, ...extra };
    await idbPut(dbKey(), ref);
    if (navigator.storage && navigator.storage.persist) navigator.storage.persist().catch(() => {});
    reference = ref;
    ui.log(T('Referenz gespeichert.', 'Reference stored.'));
  } finally {
    refBusy = false;
    $('ref-build').disabled = false;
    showRef();
  }
}

// a reference folder of the desktop version (ref/hladb)
$('ref-dir').addEventListener('change', async (ev) => {
  const want = ['hladb.json.gz', 'panel.fa', 'panel.fa.mask.bed', 'format', 'release', ...WMDA];
  const got = {};
  for (const f of ev.target.files) if (want.includes(f.name)) got[f.name] = new Uint8Array(await f.arrayBuffer());
  ev.target.value = '';
  const s = $('ref-status');
  const fmt = got.format ? parseInt(new TextDecoder().decode(got.format), 10) || 1 : 1;
  if (!got['hladb.json.gz'] || !got['panel.fa']) {
    s.className = 'status warn';
    s.textContent = T('In diesem Ordner fehlen hladb.json.gz und panel.fa. Bitte den Ordner ref/hladb wählen.', 'This folder lacks hladb.json.gz and panel.fa. Please choose the folder ref/hladb.');
    return;
  }
  if (fmt < FORMAT) {
    s.className = 'status warn';
    s.textContent = T(`Diese Referenz stammt von einer älteren Version (Format ${fmt}, nötig ${FORMAT}). Bitte „Jetzt einrichten“ nehmen.`, `This reference comes from an older version (format ${fmt}, needed ${FORMAT}). Please use "Set up now".`);
    return;
  }
  await idbPut(dbKey(), got);
  reference = got;
  showRef();
});
$('ref-clear').addEventListener('click', async () => { await idbClear(); reference = null; showRef(); });

// ── files ───────────────────────────────────────────────────────────────────
let chosen = [];
const kind = (n) => {
  const l = n.toLowerCase();
  if (l.endsWith('.bam')) return 'bam';
  if (l.endsWith('.cram')) return 'cram';
  if (l.endsWith('.bai') || l.endsWith('.csi')) return 'bai';
  if (l.endsWith('.crai')) return 'crai';
  if (l.endsWith('.fai')) return 'fai';
  if (/\.(fa|fasta|fna)$/.test(l)) return 'fasta';
  if (/\.(fa|fasta|fna)\.(gz|bgz)$/.test(l)) return 'fastagz';
  if (/\.(fq|fastq)(\.gz)?$/.test(l)) return 'fastq';
  return 'other';
};

// what the program would be given; {ok, msg, inputs, reference, referenceIndex, sample}
function plan() {
  const by = (k) => chosen.filter((f) => kind(f.name) === k);
  const [bam, cram, fq] = [by('bam'), by('cram'), by('fastq')];
  const fail = (de, en) => ({ ok: false, msg: T(de, en) });
  const strip = (n) => n.replace(/\.(bam|cram)$/i, '').replace(/[_.](R?1)?\.?(fq|fastq)(\.gz)?$/i, '').replace(/[_.]R1$/i, '');
  if (!chosen.length) return { ok: false, msg: '' };
  if (bam.length + cram.length + (fq.length ? 1 : 0) > 1) return fail('Bitte nur eine Probe auf einmal wählen.', 'Please choose one sample at a time.');
  if (bam.length) {
    const b = bam[0];
    const idx = chosen.find((f) => f.name === b.name + '.bai' || f.name === b.name + '.csi' || f.name === b.name.replace(/\.bam$/i, '.bai'));
    if (!idx) return fail(`Der Index fehlt: bitte ${b.name}.bai dazu wählen (erstellen mit samtools index).`, `The index is missing: please add ${b.name}.bai (create it with samtools index).`);
    return { ok: true, inputs: [b, idx], sample: strip(b.name) };
  }
  if (cram.length) {
    const c = cram[0];
    const idx = chosen.find((f) => f.name === c.name + '.crai');
    if (!idx) return fail(`Der Index fehlt: bitte ${c.name}.crai dazu wählen.`, `The index is missing: please add ${c.name}.crai.`);
    if (by('fastagz').length) return fail('Die Referenz-FASTA bitte entpackt wählen (.fa mit .fa.fai), nicht .fa.gz.', 'Please choose the reference FASTA unpacked (.fa with .fa.fai), not .fa.gz.');
    const fa = by('fasta')[0];
    if (!fa) return fail('Für CRAM wird die Referenz-FASTA gebraucht, mit der die Datei erstellt wurde (.fa mit .fa.fai).', 'CRAM needs the reference FASTA it was made with (.fa with .fa.fai).');
    const fai = chosen.find((f) => f.name === fa.name + '.fai');
    if (!fai) return fail(`Der FASTA-Index fehlt: bitte ${fa.name}.fai dazu wählen.`, `The FASTA index is missing: please add ${fa.name}.fai.`);
    return { ok: true, inputs: [c, idx], reference: fa, referenceIndex: fai, sample: strip(c.name) };
  }
  if (fq.length) {
    if (fq.length !== 2) return fail('Bei FASTQ bitte genau zwei Dateien wählen: R1 und R2.', 'For FASTQ please choose exactly two files: R1 and R2.');
    const [r1, r2] = [...fq].sort((a, b) => a.name.localeCompare(b.name));
    const big = r1.size + r2.size > 4e9;
    return { ok: true, inputs: [r1, r2], sample: strip(r1.name), msg: big ? T('Ganze Genome als FASTQ dauern im Browser sehr lange (Stunden). Schneller geht es mit BAM oder CRAM.', 'Whole genomes as FASTQ take very long in the browser (hours). BAM or CRAM is much faster.') : '' };
  }
  return fail('Keine BAM-, CRAM- oder FASTQ-Datei erkannt.', 'No BAM, CRAM or FASTQ file recognised.');
}

function showFiles() {
  const ul = $('file-list');
  ul.innerHTML = '';
  const label = { bam: 'BAM', cram: 'CRAM', bai: 'Index', crai: 'Index', fai: 'FASTA-Index', fasta: 'Referenz', fastagz: 'Referenz', fastq: 'FASTQ', other: '?' };
  for (const f of chosen) {
    const li = document.createElement('li');
    const k = document.createElement('b'); k.className = 'k'; k.textContent = label[kind(f.name)];
    const n = document.createElement('div'); n.textContent = f.name;
    const s = document.createElement('span'); s.textContent = f.size > 1e9 ? `${(f.size / 1e9).toFixed(1)} GB` : `${(f.size / 1e6).toFixed(1)} MB`;
    li.append(k, n, s); ul.append(li);
  }
  const p = plan();
  $('file-msg').textContent = p.msg || '';
  $('file-msg').className = p.ok ? 'note' : 'note warn';
  $('start').disabled = !p.ok || running;
}
const addFiles = (list) => {
  const seen = new Map(chosen.map((f) => [f.name, f]));
  for (const f of list) seen.set(f.name, f);
  chosen = [...seen.values()];
  showFiles();
};
$('files').addEventListener('change', (ev) => { addFiles(ev.target.files); ev.target.value = ''; });
const drop = $('drop');
drop.addEventListener('dragover', (e) => { e.preventDefault(); drop.classList.add('over'); });
drop.addEventListener('dragleave', () => drop.classList.remove('over'));
drop.addEventListener('drop', (e) => { e.preventDefault(); drop.classList.remove('over'); addFiles(e.dataTransfer.files); });

// ── run ─────────────────────────────────────────────────────────────────────
let running = false;
let t0 = 0, tick = null;
const ui = {
  phase(text, frac, busy) {
    $('progress').hidden = false;
    $('phase').textContent = text;
    $('phase').className = 'status';
    const m = $('meter').parentElement;
    m.classList.toggle('busy', !!busy);
    if (!busy && frac != null) $('meter').style.width = `${Math.round(frac * 100)}%`;
  },
  log(line) {
    const pre = $('log');
    pre.textContent += line + '\n';
    pre.scrollTop = pre.scrollHeight;
    const g = line.match(/\]\s+(\S+) done$/);
    if (g) { done.add(g[1]); ui.phase(T(`Typisierung: ${done.size} von 26 Genen fertig`, `Typing: ${done.size} of 26 genes done`), 0.35 + 0.6 * done.size / 26); }
    else if (/\[input\] (\d+) records read/.test(line)) ui.phase(T(`Lesen und Zuordnen: ${(+line.match(/(\d+) records/)[1]).toLocaleString('de-DE')} Einträge`, `Reading and mapping: ${(+line.match(/(\d+) records/)[1]).toLocaleString('en-US')} records`), null, true);
    else if (/panel mapped reads/.test(line)) ui.phase(T('Typisierung der 26 Gene …', 'Typing the 26 genes …'), 0.35);
  },
};
const done = new Set();

// the result page as written by the program; shown in the language of this page (the downloaded
// file is unchanged and has its own language switch)
let resultHtml = '';
function showResult() {
  if (!resultHtml) return;
  $('result').srcdoc = resultHtml.replace(/<html lang="(de|en)">/, `<html lang="${lang}">`).replace(/<body data-lang="(de|en)"/, `<body data-lang="${lang}"`);
}

function downloads(files, sample) {
  const box = $('downloads');
  box.innerHTML = '';
  const names = Object.keys(files).sort((a, b) => (a.endsWith('.html') ? -1 : b.endsWith('.html') ? 1 : a.localeCompare(b)));
  const mk = (name, blob) => {
    const a = document.createElement('a');
    a.className = 'btn ghost'; a.href = URL.createObjectURL(blob); a.download = name; a.textContent = name;
    return a;
  };
  const all = mk(`hla-wgs-${sample}.zip`, zipStore(Object.fromEntries(names.map((n) => [n, files[n]]))));
  all.className = 'btn';
  box.append(all, ...names.map((n) => mk(n, new Blob([files[n]]))));
}

$('start').addEventListener('click', async () => {
  const p = plan();
  if (!p.ok || running) return;
  running = true;
  $('start').disabled = true;
  $('step-out').hidden = true;
  $('log').textContent = '';
  done.clear();
  t0 = performance.now();
  tick = setInterval(() => { const s = Math.round((performance.now() - t0) / 1000); $('clock').textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`; }, 1000);
  try {
    if (!reference) await buildReference(ui);
    ui.phase(T('Lesen und Zuordnen der Reads …', 'Reading and mapping the reads …'), 0.05, true);
    const files = await job({ cmd: 'run', db: reference, inputs: p.inputs, reference: p.reference || null, referenceIndex: p.referenceIndex || null, sample: p.sample }, [], (line) => ui.log(line));
    ui.phase(T('Fertig.', 'Done.'), 1);
    $('phase').className = 'status ok';
    resultHtml = new TextDecoder().decode(files[`${p.sample}.html`]);
    showResult();
    downloads(files, p.sample);
    $('step-out').hidden = false;
    $('step-out').scrollIntoView({ behavior: 'smooth' });
  } catch (e) {
    $('phase').textContent = T('Abgebrochen: ', 'Stopped: ') + e.message;
    $('phase').className = 'status warn';
    $('meter').parentElement.classList.remove('busy');
  } finally {
    clearInterval(tick);
    running = false;
    showFiles();
  }
});

$('ref-build').addEventListener('click', async () => {
  try { await buildReference(ui); ui.phase(T('Referenz bereit.', 'Reference ready.'), 1); }
  catch (e) { $('phase').textContent = T('Abgebrochen: ', 'Stopped: ') + e.message; $('phase').className = 'status warn'; }
});

// ── ZIP (stored, no compression) ────────────────────────────────────────────
const CRC = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
const crc32 = (d) => { let c = 0xffffffff; for (let i = 0; i < d.length; i++) c = CRC[(c ^ d[i]) & 0xff] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
function zipStore(files) {
  const enc = new TextEncoder();
  const parts = [], central = [];
  let off = 0;
  const now = new Date();
  const dt = ((now.getHours() << 11) | (now.getMinutes() << 5) | (now.getSeconds() >> 1)) & 0xffff;
  const dd = (((now.getFullYear() - 1980) << 9) | ((now.getMonth() + 1) << 5) | now.getDate()) & 0xffff;
  for (const [name, data] of Object.entries(files)) {
    const n = enc.encode(name), crc = crc32(data);
    const h = new DataView(new ArrayBuffer(30));
    h.setUint32(0, 0x04034b50, true); h.setUint16(4, 20, true); h.setUint16(6, 0x0800, true); h.setUint16(8, 0, true);
    h.setUint16(10, dt, true); h.setUint16(12, dd, true); h.setUint32(14, crc, true); h.setUint32(18, data.length, true); h.setUint32(22, data.length, true);
    h.setUint16(26, n.length, true); h.setUint16(28, 0, true);
    const c = new DataView(new ArrayBuffer(46));
    c.setUint32(0, 0x02014b50, true); c.setUint16(4, 20, true); c.setUint16(6, 20, true); c.setUint16(8, 0x0800, true); c.setUint16(10, 0, true);
    c.setUint16(12, dt, true); c.setUint16(14, dd, true); c.setUint32(16, crc, true); c.setUint32(20, data.length, true); c.setUint32(24, data.length, true);
    c.setUint16(28, n.length, true); c.setUint32(42, off, true);
    parts.push(h, n, data); central.push(c, n);
    off += 30 + n.length + data.length;
  }
  const size = central.reduce((s, x) => s + x.byteLength, 0);
  const e = new DataView(new ArrayBuffer(22));
  e.setUint32(0, 0x06054b50, true); e.setUint16(8, Object.keys(files).length, true); e.setUint16(10, Object.keys(files).length, true);
  e.setUint32(12, size, true); e.setUint32(16, off, true);
  return new Blob([...parts, ...central, e], { type: 'application/zip' });
}

// ── start ───────────────────────────────────────────────────────────────────
refreshTexts = () => { showRef(); showFiles(); showResult(); };
await initWasm({ module_or_path: new URL('./pkg/hla_wgs_bg.wasm', import.meta.url) });
FORMAT = db_format();
$('ver').textContent = version();
reference = (await idbGet(dbKey()).catch(() => null)) || null;
setLang(lang);
