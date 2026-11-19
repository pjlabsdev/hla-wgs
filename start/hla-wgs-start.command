#!/bin/bash
# hla-wgs Start: double-click (macOS: .command opens in Terminal) or run `bash hla-wgs-start.command`.
# Asks for the input file; drag the file into the window (Terminal inserts its path) or type the path.
# Sets up the IPD-IMGT/HLA reference once if it is missing, runs hla-wgs with an animated progress
# display and shows the result. Long reads (PacBio HiFi): HLA_WGS_LONG=1 or the argument --long.
set -u
case "${LC_ALL:-${LC_CTYPE:-${LANG:-}}}" in *UTF-8*) ;; *) export LC_CTYPE=en_US.UTF-8 ;; esac

HLA_DAT_URL="https://github.com/ANHIG/IMGTHLA/raw/3660/hla.dat.zip"
FREQ_URL="https://raw.githubusercontent.com/pjlabsdev/hla-wgs/main/data/freq_1000g_2field.json"
HERE="$(cd "$(dirname "$0")" && pwd)"

if [ "${LANG:-}" != "${LANG#de}" ] || [ "$(defaults read -g AppleLocale 2>/dev/null | cut -c1-2)" = "de" ]; then DE=1; else DE=0; fi
tr_() { if [ "$DE" = 1 ]; then printf '%s' "$1"; else printf '%s' "$2"; fi; }

# ── display (PJ Labs: gold accent, no bold) ──────────────────────────────────
TTY=0; [ -t 1 ] && TTY=1
if [ "$TTY" = 1 ] && [ -z "${NO_COLOR:-}" ]; then
  RST=$'\e[0m'; ACC=$'\e[38;5;178m'; OK=$'\e[38;5;71m'; ERR=$'\e[38;5;167m'; MUT=$'\e[38;5;244m'; FNT=$'\e[38;5;238m'
else
  RST=""; ACC=""; OK=""; ERR=""; MUT=""; FNT=""
fi
W=64
FRAMES=(⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏)
PID=""
rep() { local o="" i=0; while [ "$i" -lt "$2" ]; do o="$o$1"; i=$((i + 1)); done; printf '%s' "$o"; }
cursor() { [ "$TTY" = 1 ] && printf '\e[?25%s' "$1"; }
title() { [ "$TTY" = 1 ] && printf '\e]0;%s\a' "$1"; }
now() { perl -MTime::HiRes=time -e 'printf "%.3f", time' 2>/dev/null || date +%s; }
secs() { awk -v a="$1" -v b="$(now)" 'BEGIN { printf "%.1fs", b - a }'; }
row() {  # row INDENT LEFT RIGHT [COLOUR_LEFT] [COLOUR_RIGHT]
  local gap=$((W - $1 - ${#2} - ${#3})); [ "$gap" -lt 1 ] && gap=1
  printf '%s%s%s%s%s%s%s%s\n' "$(rep ' ' "$1")" "${4:-}" "$2" "$RST" "$(rep ' ' "$gap")" "${5:-$MUT}" "$3" "$RST"
}
divider() { printf '  %s%s%s\n' "$FNT" "$(rep ─ $((W - 2)))" "$RST"; }
result() {  # result ok|fail LABEL DETAIL TIME
  local sym="✓" col="$OK" l="$2"; [ "$1" = fail ] && sym="✗" && col="$ERR"
  [ -n "$3" ] && l="$l  $3"
  local gap=$((W - 6 - ${#l} - ${#4})); [ "$gap" -lt 1 ] && gap=1
  local text="$2"; [ -n "$3" ] && text="$text  $MUT$3$RST"; [ "$1" = fail ] && text="$ERR$2${3:+  $3}$RST"
  printf '    %s%s%s %s%s%s%s%s\n' "$col" "$sym" "$RST" "$text" "$(rep ' ' "$gap")" "$MUT" "$4" "$RST"
}
spin() { [ "$TTY" = 1 ] && printf '\r\e[2K    %s%s%s %s' "$ACC" "${FRAMES[$(($1 % 10))]}" "$RST" "$2"; }
clearline() { [ "$TTY" = 1 ] && printf '\r\e[2K'; }
bar() {  # bar DONE TOTAL WIDTH
  local n=0; [ "$2" -gt 0 ] && n=$(($1 * $3 / $2)); [ "$n" -gt "$3" ] && n=$3
  printf '%s%s%s%s%s' "$ACC" "$(rep ━ "$n")" "$FNT" "$(rep ━ $(($3 - n)))" "$RST"
}
group() { local sep=","; [ "$DE" = 1 ] && sep="."; printf '%s' "$1" | perl -pe "1 while s/^(\\d+)(\\d{3})/\$1$sep\$2/"; }
ask() { printf '\n  %s\n  %s>%s ' "$(tr_ "$1" "$2")" "$ACC" "$RST"; }
finish() { cursor h; printf '\n'; printf '  %s%s%s' "$MUT" "$(tr_ "Enter zum Schließen" "Press Enter to close")" "$RST"; read -r _; exit "${1:-0}"; }
trap 'cursor h' EXIT
trap 'cursor h; [ -n "$PID" ] && kill "$PID" 2>/dev/null; exit 130' INT TERM HUP

# runs a command in the background with a spinner: step LABEL COMMAND …  (sets T0 for the result line)
step() {
  local label=$1; shift
  T0=$(now)
  if [ "$TTY" = 1 ]; then
    "$@" >"$WORKLOG" 2>&1 & PID=$!; local i=0
    cursor l
    while kill -0 "$PID" 2>/dev/null; do spin "$i" "$label"; i=$((i + 1)); sleep 0.08; done
    wait "$PID"; local rc=$?; PID=""; clearline; return $rc
  else
    "$@" >"$WORKLOG" 2>&1
  fi
}

# download with progress bar: download URL TARGET
download() {
  T0=$(now)
  if [ "$TTY" = 1 ]; then
    local total; total=$(curl -fsSLI "$1" 2>/dev/null | awk 'tolower($1)=="content-length:"{v=$2} END{gsub("\r","",v); print v+0}')
    curl -fsSL -o "$2" "$1" 2>/dev/null & PID=$!; local i=0 size
    cursor l
    while kill -0 "$PID" 2>/dev/null; do
      size=$(stat -f %z "$2" 2>/dev/null || stat -c %s "$2" 2>/dev/null || echo 0)
      if [ "${total:-0}" -gt 0 ]; then
        spin "$i" "Download  $(bar "$size" "$total" 24)  $MUT$(awk -v s="$size" -v t="$total" 'BEGIN{printf "%5.1f / %.1f MB", s/1048576, t/1048576}')$RST"
      else
        spin "$i" "Download  $MUT$(awk -v s="$size" 'BEGIN{printf "%.1f MB", s/1048576}')$RST"
      fi
      i=$((i + 1)); sleep 0.1
    done
    wait "$PID"; local rc=$?; PID=""; clearline; return $rc
  else
    curl -fsSL -o "$2" "$1"
  fi
}

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
    printf '  %s%s %s%s\n' "$ERR" "$(tr_ "Datei nicht gefunden:" "File not found:")" "$p" "$RST"
  done
}

# hla-wgs program: next to this file, in the repository build, or in PATH
BIN=""
for c in "$HERE/hla-wgs" "$HERE/bin/hla-wgs" "$HERE/../target/release/hla-wgs" "$(command -v hla-wgs 2>/dev/null)"; do
  if [ -n "$c" ] && [ -x "$c" ]; then BIN="$c"; break; fi
done
WORKLOG="$(mktemp -t hla-wgs-start.XXXXXX)"

# ── header ───────────────────────────────────────────────────────────────────
clear 2>/dev/null
title "hla-wgs"
printf '\n'
row 2 "hla-wgs" "$(date '+%d.%m.%Y  %H:%M')" "$ACC" "$MUT"
printf '  %s%s%s\n' "$MUT" "$(tr_ "HLA-Typisierung aus dem Genom · nur für Forschung (RUO)" "HLA typing from the genome · research use only (RUO)")" "$RST"
divider
if [ -z "$BIN" ]; then
  printf '\n  %s%s%s\n' "$ERR" "$(tr_ "Das Programm hla-wgs wurde nicht gefunden (neben dieser Datei, in ../target/release oder im PATH)." "The hla-wgs program was not found (next to this file, in ../target/release or in PATH).")" "$RST"
  finish 1
fi

# ── reference: HLA_WGS_DB, else an existing one, else ~/hla-wgs-ref/hladb ─────
DB="${HLA_WGS_DB:-}"
if [ -z "$DB" ]; then
  for c in "$HOME/Downloads/hla-wgs-ref/hladb" "$HOME/hla-wgs-ref/hladb"; do
    if [ -f "$c/hladb.json.gz" ]; then DB="$c"; break; fi
  done
fi
[ -z "$DB" ] && DB="$HOME/hla-wgs-ref/hladb"

if [ ! -f "$DB/hladb.json.gz" ] || [ ! -f "$DB/panel.fa" ]; then
  WORK="$(dirname "$DB")"; mkdir -p "$DB"
  printf '\n'
  row 2 "$(tr_ "Einmalige Einrichtung" "One-time setup")" "IPD-IMGT/HLA 3.66" "$ACC"
  printf '  %s%s%s\n' "$MUT" "$DB" "$RST"
  askpath "hla.dat hineinziehen oder Pfad eingeben, oder nur Enter zum Herunterladen (etwa 45 MB):" "Drop hla.dat or enter its path, or just press Enter to download it (about 45 MB):" empty
  DAT="$REPLY_PATH"
  printf '\n'
  if [ -z "$DAT" ]; then
    download "$HLA_DAT_URL" "$WORK/hla.dat.zip" || { result fail "Download" "" "$(secs "$T0")"; finish 1; }
    result ok "Download" "$(awk -v s="$(stat -f %z "$WORK/hla.dat.zip" 2>/dev/null || echo 0)" 'BEGIN{printf "%.1f MB", s/1048576}')" "$(secs "$T0")"
    step "$(tr_ "Entpacken" "Unpacking")" unzip -o -q "$WORK/hla.dat.zip" -d "$WORK" || { result fail "$(tr_ "Entpacken" "Unpacking")" "" "$(secs "$T0")"; finish 1; }
    result ok "$(tr_ "Entpacken" "Unpacking")" "hla.dat" "$(secs "$T0")"
    DAT="$WORK/hla.dat"
  fi
  FREQ="$HERE/freq_1000g_2field.json"
  [ -f "$FREQ" ] || FREQ="$HERE/../data/freq_1000g_2field.json"
  if [ ! -f "$FREQ" ]; then
    FREQ="$WORK/freq_1000g_2field.json"
    [ -f "$FREQ" ] || curl -fsL -o "$FREQ" "$FREQ_URL" || { result fail "$(tr_ "Häufigkeitstabelle" "Frequency table")" "" ""; finish 1; }
  fi
  step "$(tr_ "Datenbank bauen" "Building the database")" "$BIN" build-db "$DAT" "$FREQ" "$DB" || { result fail "$(tr_ "Datenbank" "Database")" "" "$(secs "$T0")"; cat "$WORKLOG"; finish 1; }
  result ok "$(tr_ "Datenbank" "Database")" "build-db" "$(secs "$T0")"
fi
if [ ! -f "$DB/panel.fa.mask.bed" ]; then
  askpath "Wiederholungsmaske (einmalig): Genom GRCh38 als FASTA hineinziehen, oder Enter zum Überspringen:" "Repeat mask (once): drop the GRCh38 genome FASTA, or press Enter to skip:" empty
  if [ -n "$REPLY_PATH" ]; then
    printf '\n'
    if step "$(tr_ "Wiederholungsmaske bauen" "Building the repeat mask")" "$BIN" build-mask "$DB" "$REPLY_PATH"; then
      result ok "$(tr_ "Wiederholungsmaske" "Repeat mask")" "build-mask" "$(secs "$T0")"
    else
      result fail "$(tr_ "Wiederholungsmaske" "Repeat mask")" "$(tr_ "weiter ohne Maske" "continuing without")" "$(secs "$T0")"
    fi
  fi
fi

# ── input ────────────────────────────────────────────────────────────────────
askpath "BAM, CRAM oder FASTQ (R1) hier hineinziehen oder Pfad eingeben, dann Enter:" "Drop a BAM, CRAM or FASTQ (R1) here or enter its path, then press Enter:"
IN="$REPLY_PATH"
lower="$(printf '%s' "$IN" | tr '[:upper:]' '[:lower:]')"
EXTRA=(); LONG=0; IN2=""
stop() { printf '  %s%s%s\n' "$ERR" "$1" "$RST"; finish 1; }
case "$lower" in
  *.bam)
    if [ ! -f "$IN.bai" ] && [ ! -f "${IN%.*}.bai" ] && [ ! -f "$IN.csi" ]; then stop "$(tr_ "Zur BAM fehlt der Index (.bai daneben)." "The BAM index (.bai) is missing.")"; fi ;;
  *.cram)
    [ -f "$IN.crai" ] || stop "$(tr_ "Zur CRAM fehlt der Index (.crai daneben)." "The CRAM index (.crai) is missing.")" ;;
  *.fq|*.fastq|*.fq.gz|*.fastq.gz) ;;
  *) stop "$(tr_ "Unbekanntes Format. Erlaubt sind BAM, CRAM oder FASTQ." "Unknown format. Allowed are BAM, CRAM or FASTQ.")" ;;
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
T="$( (sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4) | head -1)"
SHOWOUT="$OUT"; case "$OUT" in "$HOME"/*) SHOWOUT="~/${OUT#"$HOME"/}" ;; esac

printf '\n'
divider
row 2 "$(tr_ "Probe" "Sample")" "$S" "$MUT" ""
row 2 "$(tr_ "Eingabe" "Input")" "$(basename "$IN")" "$MUT" ""
row 2 "$(tr_ "Ausgabe" "Output")" "$SHOWOUT" "$MUT" ""
row 2 "Threads" "$T$( [ "$LONG" = 1 ] && printf '  ·  HiFi')" "$MUT" ""
divider
ask "Start mit Enter (oder Strg-C zum Abbrechen)" "Press Enter to start (or Ctrl-C to cancel)"; read -r _ || exit 1
mkdir -p "$OUT" || finish 1
LOG="$OUT/$S.log"

# ── run with progress ────────────────────────────────────────────────────────
printf '\n'
title "hla-wgs · $S"
[ "$DE" = 1 ] && export HLA_WGS_LANG=de   # result page PREFIX.html in German
START=$(now)
if [ "$LONG" = 1 ]; then
  "$BIN" long "$DB" "$IN" "$OUT/$S" "$T" >"$LOG" 2>&1 & PID=$!; TAB="$OUT/$S.long.tsv"
elif [ -n "$IN2" ]; then
  "$BIN" "$DB" "$IN" "$IN2" "$OUT/$S" "$T" >"$LOG" 2>&1 & PID=$!; TAB="$OUT/$S.conf.tsv"
else
  "$BIN" "$DB" "$IN" "$OUT/$S" "$T" ${EXTRA[@]+"${EXTRA[@]}"} >"$LOG" 2>&1 & PID=$!; TAB="$OUT/$S.conf.tsv"
fi

# phases from the log: reading ([input] or "panel mapped reads"), typing (one "groups" and one
# "done" line per gene), writing (after "total")
L_READ="$(tr_ "Reads lesen" "Reading reads")"; L_READ_OK="$(tr_ "Reads gelesen" "Reads read")"
L_TYPE="$(tr_ "Typisierung" "Typing")"; L_WRITE="$(tr_ "Ergebnis schreiben" "Writing results")"
phase=1; i=0; T0=$START; cursor l
while kill -0 "$PID" 2>/dev/null; do
  if [ "$phase" = 1 ] && grep -q -e '^\[input\]' -e 'panel mapped reads' "$LOG" 2>/dev/null; then
    clearline
    pairs="$(sed -n 's/.* \([0-9][0-9]*\) pairs.*/\1/p' "$LOG" | head -1)"
    if [ -n "$pairs" ]; then result ok "$L_READ_OK" "$(group "$pairs") $(tr_ "Paare" "pairs")" "$(secs "$T0")"
    else result ok "$L_READ_OK" "" "$(secs "$T0")"; fi
    phase=2; T0=$(now)
  fi
  if [ "$phase" = 2 ]; then
    genes=$(grep -c ' groups \[' "$LOG" 2>/dev/null); done_=$(grep -c ' done$' "$LOG" 2>/dev/null)
    last="$(grep ' done$' "$LOG" 2>/dev/null | tail -1 | awk '{print $(NF-1)}')"
    if grep -q '\] total$' "$LOG" 2>/dev/null; then
      clearline; result ok "$L_TYPE" "$done_ $(tr_ "Gene" "genes")" "$(secs "$T0")"; phase=3; T0=$(now)
    else
      spin "$i" "$L_TYPE  $(bar "$done_" "${genes:-0}" 24)  $MUT$done_ / ${genes:-0}  $last$RST"
    fi
  elif [ "$phase" = 1 ]; then
    spin "$i" "$L_READ  $MUT$(secs "$T0")$RST"
  else
    spin "$i" "$L_WRITE"
  fi
  i=$((i + 1)); sleep 0.1
done
wait "$PID"; rc=$?; PID=""; clearline; cursor h
if [ "$rc" = 0 ]; then
  [ "$phase" = 1 ] && result ok "$L_READ_OK" "" "$(secs "$T0")"
  [ "$phase" -le 2 ] && result ok "$L_TYPE" "" "$(secs "$T0")"
  result ok "$(tr_ "Ergebnis geschrieben" "Results written")" "$(basename "$TAB")" "$(secs "$T0")"
  [ -f "$OUT/$S.html" ] && result ok "$(tr_ "Ergebnisseite" "Result page")" "$S.html" ""
fi

printf '\n'
divider
if [ "$rc" != 0 ]; then
  row 2 "$(tr_ "Fehler $rc" "Error $rc")" "$(secs "$START")" "$ERR"
  tail -3 "$LOG" | sed "s/^/    $MUT/; s/\$/$RST/"
  printf '    %s%s %s%s\n' "$MUT" "$(tr_ "Protokoll:" "Log:")" "$LOG" "$RST"
  finish "$rc"
fi
row 2 "$(tr_ "Fertig" "Done")" "$(secs "$START")" "$OK"
printf '\n'
cols() { if command -v column >/dev/null 2>&1; then column -t -s "$(printf '\t')"; else cat; fi; }
# genes estimated at 0 copies (PREFIX.tsv, column copies) are absent: the table lists them as such
# instead of the nearest database allele that PREFIX.conf.tsv still carries for them
ABSENT=""; [ -f "$OUT/$S.tsv" ] && [ "$LONG" = 0 ] && ABSENT="$(awk -F'\t' 'NR > 1 && $8 == "0" { printf "%s ", $1 }' "$OUT/$S.tsv")"
cut -f1-5 "$TAB" | awk -F'\t' -v a=" $ABSENT" -v no="$(tr_ "nicht vorhanden" "absent")" 'NR == 1 { print; next }
  index(a, " " $1 " ") { next } { print }
  END { n = split(a, x, " "); for (k = 1; k <= n; k++) print x[k] "\t-\t" no "\t-\t0" }' |
  cols | awk -v m="$MUT" -v r="$RST" -v g="$OK" 'NR==1 { print "  " m $0 r; next } { line = "  " $0; if ($NF == "4") sub(/4$/, g "4" r, line); print line }'
printf '\n  %s%s%s\n' "$MUT" "$(tr_ "safe_call und resolution: so viele Felder, wie die Reads tragen. Maßgeblich ist der sichere Ruf." "safe_call and resolution: as many fields as the reads support. Use the safe call.")" "$RST"
printf '  %s%s%s\n' "$MUT" "$SHOWOUT" "$RST"
title "hla-wgs · $S · $(tr_ "fertig" "done")"
if command -v open >/dev/null 2>&1; then open "$OUT"; [ -f "$OUT/$S.html" ] && open "$OUT/$S.html"; fi
finish 0
