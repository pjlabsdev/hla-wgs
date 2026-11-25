// hla-wgs in a Web Worker: one worker per job (the program keeps state for one run).
// Messages in:  {cmd: 'build-db', dat: Blob, freq: ArrayBuffer}
//               {cmd: 'run', db: {name: Uint8Array}, inputs: [File], reference: File|null, sample}
// Messages out: {type: 'log', line} … then {type: 'done', files: {name: Uint8Array}} or {type: 'error', message}
import init, * as hla from './pkg/hla_wgs.js';

const ready = init({ module_or_path: new URL('./pkg/hla_wgs_bg.wasm', import.meta.url) }).then(() => hla.init());

function collect(dir) {
  const files = {};
  for (const p of hla.list_files(dir)) files[p.slice(dir.length + 1)] = hla.take_file(p);
  return files;
}

onmessage = async (ev) => {
  const m = ev.data;
  let error = '';
  try {
    await ready;
    hla.set_log((line) => {
      if (line.startsWith('error: ')) error = line.slice(7);
      postMessage({ type: 'log', line });
    });
    let files;
    if (m.cmd === 'build-db') {
      hla.put_blob('/src/hla.dat', m.dat);
      hla.put_file('/src/freq.json', new Uint8Array(m.freq));
      hla.run(['hla-wgs', 'build-db', '/src/hla.dat', '/src/freq.json', '/db']);
      files = collect('/db');
    } else if (m.cmd === 'run') {
      for (const [name, data] of Object.entries(m.db)) hla.put_file('/db/' + name, data);
      for (const f of m.inputs) hla.put_blob('/in/' + f.name, f);
      if (m.reference) {
        hla.put_blob('/ref/' + m.reference.name, m.reference);
        hla.put_blob('/ref/' + m.reference.name + '.fai', m.referenceIndex);
      }
      const main = m.inputs.filter((f) => !/\.(bai|csi|crai)$/i.test(f.name)).map((f) => '/in/' + f.name);
      const args = ['hla-wgs', '/db', ...main, '/out/' + m.sample, '1'];
      if (m.reference) args.push('/ref/' + m.reference.name);
      hla.run(args);
      files = collect('/out');
    }
    postMessage({ type: 'done', files }, Object.values(files).map((f) => f.buffer));
  } catch (e) {
    postMessage({ type: 'error', message: error || String(e && e.message || e) });
  }
};
