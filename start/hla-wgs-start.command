#!/bin/bash
# hla-wgs Start: double-click (macOS: .command opens in Terminal) or run `bash hla-wgs-start.command`.
# Asks for the input file; drag the file into the window (Terminal inserts its path) or type the path.
# Sets up the IPD-IMGT/HLA reference once if it is missing, runs hla-wgs and shows the result.
set -u

HLA_DAT_URL="https://github.com/ANHIG/IMGTHLA/raw/3660/hla.dat.zip"
FREQ_URL="https://raw.githubusercontent.com/pjlabsdev/hla-wgs/main/data/freq_1000g_2field.json"
HERE="$(cd "$(dirname "$0")" && pwd)"

if [ "${LANG:-}" != "${LANG#de}" ] || [ "$(defaults read -g AppleLocale 2>/dev/null | cut -c1-2)" = "de" ]; then DE=1; else DE=0; fi
say() { if [ "$DE" = 1 ]; then printf '%s\n' "$1"; else printf '%s\n' "$2"; fi; }
ask() { if [ "$DE" = 1 ]; then printf '\n%s\n> ' "$1"; else printf '\n%s\n> ' "$2"; fi; }
finish() { printf '\n'; say "Enter zum Schließen." "Press Enter to close."; read -r _; exit "${1:-0}"; }

# a path as dragged into Terminal: quotes or backslash escapes, trailing space
clean() {
  local p="$1"
  p="${p#"${p%%[![:space:]]*}"}"; p="${p%"${p##*[![:space:]]}"}"
  case "$p" in \'*\') p="${p#\'}"; p="${p%\'}" ;; \"*\") p="${p#\"}"; p="${p%\"}" ;; *) p="$(printf '%s' "$p" | sed 's/\\\(.\)/\1/g')" ;; esac
  case "$p" in "~/"*) p="$HOME/${p#\~/}" ;; esac
  printf '%s' "$p"
}

askpath() {  # $1 de, $2 en, $3 allow empty
  local p
  while true; do
    ask "$1" "$2"; IFS= read -r p || finish 1
    p="$(clean "$p")"
    if [ -z "$p" ] && [ "${3:-}" = empty ]; then REPLY_PATH=""; return; fi
    if [ -f "$p" ]; then REPLY_PATH="$p"; return; fi
    say "Datei nicht gefunden: $p" "File not found: $p"
  done
}

# hla-wgs program: next to this file, in the repository build, or in PATH
BIN=""
for c in "$HERE/hla-wgs" "$HERE/bin/hla-wgs" "$HERE/../target/release/hla-wgs" "$(command -v hla-wgs 2>/dev/null)"; do
  if [ -n "$c" ] && [ -x "$c" ]; then BIN="$c"; break; fi
done
clear 2>/dev/null
say "hla-wgs · HLA-Typisierung aus dem Genom" "hla-wgs · HLA typing from the genome"
say "Nur für Forschung (RUO). Ersetzt keine Bestätigung durch ein akkreditiertes HLA-Labor." "Research use only (RUO). Does not replace confirmation by an accredited HLA laboratory."
if [ -z "$BIN" ]; then
  say "Das Programm hla-wgs wurde nicht gefunden (neben dieser Datei, in ../target/release oder im PATH)." "The hla-wgs program was not found (next to this file, in ../target/release or in PATH)."
  finish 1
fi

# reference: HLA_WGS_DB, else an existing one, else ~/hla-wgs-ref/hladb
DB="${HLA_WGS_DB:-}"
if [ -z "$DB" ]; then
  for c in "$HOME/Downloads/hla-wgs-ref/hladb" "$HOME/hla-wgs-ref/hladb"; do
    if [ -f "$c/hladb.json.gz" ]; then DB="$c"; break; fi
  done
fi
[ -z "$DB" ] && DB="$HOME/hla-wgs-ref/hladb"

if [ ! -f "$DB/hladb.json.gz" ] || [ ! -f "$DB/panel.fa" ]; then
  WORK="$(dirname "$DB")"; mkdir -p "$DB"
  say "" ""
  say "Einmalige Einrichtung: Referenz aus IPD-IMGT/HLA 3.66 nach $DB (einige Minuten)." "One-time setup: reference from IPD-IMGT/HLA 3.66 into $DB (a few minutes)."
  askpath "hla.dat hineinziehen oder Pfad eingeben, oder nur Enter zum Herunterladen (etwa 45 MB):" "Drop hla.dat or enter its path, or just press Enter to download it (about 45 MB):" empty
  DAT="$REPLY_PATH"
  if [ -z "$DAT" ]; then
    curl -fL --retry 3 -o "$WORK/hla.dat.zip" "$HLA_DAT_URL" && unzip -o -q "$WORK/hla.dat.zip" -d "$WORK" || { say "Download fehlgeschlagen." "Download failed."; finish 1; }
    DAT="$WORK/hla.dat"
  fi
  FREQ="$HERE/freq_1000g_2field.json"
  [ -f "$FREQ" ] || FREQ="$HERE/../data/freq_1000g_2field.json"
  if [ ! -f "$FREQ" ]; then
    FREQ="$WORK/freq_1000g_2field.json"
    [ -f "$FREQ" ] || curl -fsL -o "$FREQ" "$FREQ_URL" || { say "Häufigkeitstabelle nicht ladbar." "Cannot fetch the frequency table."; finish 1; }
  fi
  "$BIN" build-db "$DAT" "$FREQ" "$DB" || { say "build-db fehlgeschlagen." "build-db failed."; finish 1; }
fi
if [ ! -f "$DB/panel.fa.mask.bed" ]; then
  askpath "Wiederholungsmaske (einmalig): Genom GRCh38 als FASTA hineinziehen, oder Enter zum Überspringen:" "Repeat mask (once): drop the GRCh38 genome FASTA, or press Enter to skip:" empty
  if [ -n "$REPLY_PATH" ]; then "$BIN" build-mask "$DB" "$REPLY_PATH" || say "build-mask fehlgeschlagen, es geht ohne Maske weiter." "build-mask failed, continuing without the mask."; fi
fi

# input
askpath "BAM, CRAM oder FASTQ (R1) hier hineinziehen oder Pfad eingeben, dann Enter:" "Drop a BAM, CRAM or FASTQ (R1) here or enter its path, then press Enter:"
IN="$REPLY_PATH"
lower="$(printf '%s' "$IN" | tr '[:upper:]' '[:lower:]')"
EXTRA=(); LONG=0
case "$lower" in
  *.bam)
    if [ ! -f "$IN.bai" ] && [ ! -f "${IN%.*}.bai" ] && [ ! -f "$IN.csi" ]; then say "Zur BAM fehlt der Index (.bai daneben)." "The BAM index (.bai) is missing."; finish 1; fi ;;
  *.cram)
    [ -f "$IN.crai" ] || { say "Zur CRAM fehlt der Index (.crai daneben)." "The CRAM index (.crai) is missing."; finish 1; } ;;
  *.fq|*.fastq|*.fq.gz|*.fastq.gz) ;;
  *) say "Unbekanntes Format. Erlaubt sind BAM, CRAM oder FASTQ." "Unknown format. Allowed are BAM, CRAM or FASTQ."; finish 1 ;;
esac
# long reads (PacBio HiFi) only on request: HLA_WGS_LONG=1 or the argument --long
if [ "${HLA_WGS_LONG:-0}" = 1 ] || [ "${1:-}" = "--long" ]; then LONG=1; fi
if [ "$LONG" = 0 ]; then
  case "$lower" in
    *.cram) askpath "Referenz der CRAM (GRCh38-FASTA mit .fai) hineinziehen:" "Drop the reference of the CRAM (GRCh38 FASTA with .fai):"; EXTRA=("$REPLY_PATH") ;;
    *.fq|*.fastq|*.fq.gz|*.fastq.gz) askpath "Zweite FASTQ-Datei (R2) hineinziehen:" "Drop the second FASTQ file (R2):"; IN2="$REPLY_PATH" ;;
  esac
fi

# sample name and an output folder that holds no earlier result
S="$(basename "$IN")"
for x in .fastq.gz .fq.gz .fastq .fq .bam .cram .BAM .CRAM; do S="${S%"$x"}"; done
for x in _R1 _1 .R1 _r1; do S="${S%"$x"}"; done
[ -z "$S" ] && S=sample
OUT="$HOME/Downloads/hla-wgs-$S"; n=2
while [ -e "$OUT/$S.tsv" ] || [ -e "$OUT/$S.long.tsv" ]; do OUT="$HOME/Downloads/hla-wgs-$S-$n"; n=$((n + 1)); done
mkdir -p "$OUT"
T="$( (sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) | head -1)"

say "" ""
say "Eingabe:  $IN" "Input:    $IN"
say "Ausgabe:  $OUT" "Output:   $OUT"
say "Threads:  $T" "Threads:  $T"
ask "Start mit Enter (oder Strg-C zum Abbrechen)" "Press Enter to start (or Ctrl-C to cancel)"; read -r _

start=$(date +%s)
if [ "$LONG" = 1 ]; then
  "$BIN" long "$DB" "$IN" "$OUT/$S" "$T" 2>&1 | tee "$OUT/$S.log"; rc=${PIPESTATUS[0]}; TAB="$OUT/$S.long.tsv"
elif [ -n "${IN2:-}" ]; then
  "$BIN" "$DB" "$IN" "$IN2" "$OUT/$S" "$T" 2>&1 | tee "$OUT/$S.log"; rc=${PIPESTATUS[0]}; TAB="$OUT/$S.conf.tsv"
else
  "$BIN" "$DB" "$IN" "$OUT/$S" "$T" ${EXTRA[@]+"${EXTRA[@]}"} 2>&1 | tee "$OUT/$S.log"; rc=${PIPESTATUS[0]}; TAB="$OUT/$S.conf.tsv"
fi
secs=$(( $(date +%s) - start ))

printf '\n'
if [ "$rc" != 0 ]; then say "hla-wgs ist mit Fehler $rc beendet worden. Protokoll: $OUT/$S.log" "hla-wgs stopped with error $rc. Log: $OUT/$S.log"; finish "$rc"; fi
say "Fertig in $secs s. Ergebnis ($TAB):" "Done in $secs s. Result ($TAB):"
printf '\n'
cut -f1-5 "$TAB" | column -t -s "$(printf '\t')"
printf '\n'
say "safe_call und resolution: so viele Felder, wie die Reads tragen. Maßgeblich ist der sichere Ruf." "safe_call and resolution: as many fields as the reads support. Use the safe call."
command -v open >/dev/null 2>&1 && open "$OUT"
finish 0
