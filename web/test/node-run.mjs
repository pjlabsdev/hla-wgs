// Runs the browser build (pkg/) in Node with every file in memory, for the comparison with the
// native program: node test/node-run.mjs DBDIR INPUT OUTPREFIX [REF.fa]
// The input files (and the reference) are handed over as file-like objects that are read in
// pieces, as in the browser (FileReaderSync is stood in for by reads of the file).
import { readFileSync, writeFileSync, readdirSync, statSync, openSync, readSync } from 'node:fs';
import { basename, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const w = await import(join(here, '..', 'pkg', 'hla_wgs.js'));
w.initSync({ module: readFileSync(join(here, '..', 'pkg', 'hla_wgs_bg.wasm')) });
w.init();
const [db, input, out, ref] = process.argv.slice(2);
for (const f of readdirSync(db)) if (statSync(join(db, f)).isFile()) w.put_file('/db/' + f, readFileSync(join(db, f)));
const name = basename(input);
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
const put = (p, as) => { try { statSync(p); w.put_blob(as, new LocalFile(p)); } catch { } };
put(input, '/in/' + name);
for (const ext of ['.bai', '.csi', '.crai']) put(input + ext, '/in/' + name + ext);
if (input.endsWith('.bam')) put(input.slice(0, -4) + '.bai', '/in/' + name.slice(0, -4) + '.bai');
if (ref) { put(ref, '/ref/' + basename(ref)); put(ref + '.fai', '/ref/' + basename(ref) + '.fai'); }
const t0 = Date.now();
let err = '';
w.set_log((s) => { if (s.startsWith('error: ')) err = s; process.stderr.write(s + '\n'); });
const args = ['hla-wgs', '/db', '/in/' + name, '/out/' + basename(out), '1'];
if (ref) args.push('/ref/' + basename(ref));
try { w.run(args); } catch (e) { console.error(err || String(e)); process.exit(1); }
for (const p of w.list_files('/out')) writeFileSync(join(dirname(out), basename(p)), w.take_file(p));
console.error(`wasm run ${((Date.now() - t0) / 1000).toFixed(1)} s, peak rss ${(process.resourceUsage().maxRSS / 1024).toFixed(0)} MB`);
