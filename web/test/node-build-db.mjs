// build-db of the browser build in Node: node test/node-build-db.mjs hla.dat freq.json OUTDIR
import { readFileSync, writeFileSync, mkdirSync, statSync, openSync, readSync } from 'node:fs';
import { join, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const w = await import(join(here, '..', 'pkg', 'hla_wgs.js'));
w.initSync({ module: readFileSync(join(here, '..', 'pkg', 'hla_wgs_bg.wasm')) });
w.init();
class LocalFile {
  constructor(path, start = 0, end = statSync(path).size) { this.path = path; this.start = start; this.end = end; this.size = end - start; }
  slice(a, b) { return new LocalFile(this.path, this.start + a, this.start + Math.min(b, this.size)); }
}
globalThis.FileReaderSync = class {
  readAsArrayBuffer(f) {
    const buf = Buffer.alloc(f.size); const fd = openSync(f.path, 'r');
    let n = 0; while (n < f.size) { const k = readSync(fd, buf, n, f.size - n, f.start + n); if (k === 0) break; n += k; }
    return buf.buffer.slice(buf.byteOffset, buf.byteOffset + n);
  }
};
const [dat, freq, out] = process.argv.slice(2);
w.put_blob('/src/hla.dat', new LocalFile(dat));
w.put_file('/src/freq.json', readFileSync(freq));
let err = '';
w.set_log((s) => { if (s.startsWith('error: ')) err = s; process.stderr.write(s + '\n'); });
const t0 = Date.now();
try { w.run(['hla-wgs', 'build-db', '/src/hla.dat', '/src/freq.json', '/db']); } catch (e) { console.error(err || String(e)); process.exit(1); }
mkdirSync(out, { recursive: true });
for (const p of w.list_files('/db')) writeFileSync(join(out, basename(p)), w.take_file(p));
console.error(`wasm build-db ${((Date.now() - t0) / 1000).toFixed(1)} s`);
