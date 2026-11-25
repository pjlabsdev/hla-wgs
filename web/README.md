# hla-wgs in the browser

The same program as the command-line version, compiled to WebAssembly. It runs completely on the
user's computer: BAM, CRAM and FASTQ files are read locally in pieces (for BAM and CRAM only the HLA
region on chromosome 6 and the unmapped reads, through the index) and are never uploaded. The page
needs no server-side code; any static web host will do.

## What the page does

1. **Reference.** The first time, it downloads `hla.dat.zip` of IPD-IMGT/HLA 3.66 and the WMDA files
   directly from `raw.githubusercontent.com/ANHIG/IMGTHLA` (unchanged; the licence CC BY-ND does not
   allow serving a built reference), unpacks it with the browser's `DecompressionStream` and runs
   `build-db` in a Web Worker. The result is stored in IndexedDB under `hladb-3.66-f<format>` and
   used from then on. Alternatively the user can choose the folder `ref/hladb` of the desktop version.
2. **Input.** BAM with `.bai`/`.csi`, CRAM with `.crai` plus the reference FASTA (`.fa` with `.fa.fai`,
   not bgzip-compressed), or FASTQ R1 and R2.
3. **Run.** A fresh Web Worker runs the program with the same arguments as on the command line
   (`hla-wgs /db /in/SAMPLE.bam /out/SAMPLE 1`). Progress lines appear on the page.
4. **Result.** `SAMPLE.html` is shown on the page; all result files can be downloaded one by one or as
   a ZIP.

## How it is built

`web/Cargo.toml` compiles `../src/main.rs` as a `cdylib` for `wasm32-unknown-unknown`. The typing code
is not duplicated: `src/vfs.rs` gives it file access, a clock and progress output that are the file
system, `std::time` and stderr natively, and registered in-memory files, the user's local files
(`FileReaderSync`), `performance.now()` and a JavaScript callback in the browser. `src/web.rs` holds
the functions the page calls. In the browser the program runs on one thread (rayon falls back to the
calling thread; the input reader feeds the mapper directly instead of through a second thread).

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.106
web/build.sh                 # -> web/dist (about 3 MB)
python3 -m http.server -d web/dist 8765
```

`web/Cargo.toml` must carry the same version as `Cargo.toml` (it is printed into every result);
`build.sh` refuses to build otherwise.

## Same result as the native program

Checked on NA12878 (1000 Genomes 30x, MHC region and unmapped reads cut out of the NYGC CRAM:
157 MB BAM, 79 MB CRAM), database IPD-IMGT/HLA 3.66:

| | native, 1 thread | browser build |
|---|---|---|
| BAM, in Node | 77 s | 126 s |
| BAM, in Chromium (whole page) | | 130 s |
| CRAM with 3.2 GB reference FASTA, in Node | | 173 s |
| `build-db` | 62 s (4 threads), 214 s CPU | 445 s |

`.tsv`, `.conf.tsv`, `.novel.tsv`, `.novel.fa`, `.html` and `hladb.json.gz`/`panel.fa` are
byte-identical to the native program; `.json` differs only in `analysis_time`.

```bash
node web/test/node-run.mjs DBDIR SAMPLE.bam OUT/SAMPLE             # browser build in Node
node web/test/node-run.mjs DBDIR SAMPLE.cram OUT/SAMPLE REF.fa
node web/test/node-build-db.mjs hla.dat data/freq_1000g_2field.json OUTDIR
node web/test/browser.cjs http://localhost:8765/ OUTDIR DBDIR SAMPLE.bam SAMPLE.bam.bai   # Chromium, Playwright
```

## Limits of this version

- One thread: about 1.7 times the time of the native program on one thread, and slower than the
  native program on several cores. Threads in WebAssembly (`wasm-bindgen-rayon`) would need the page
  to be served with the headers `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp`.
- Building the reference takes 5 to 10 minutes once per browser.
- Whole-genome FASTQ works but takes hours; BAM or CRAM is the way for whole genomes.
- PacBio HiFi (`hla-wgs long`) is not offered on the page yet.
- Memory: WebAssembly has at most 4 GB; the NA12878 run above peaked at about 1.1 GB (Node process in total).
