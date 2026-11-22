#!/bin/bash
# hla-wgs installer for macOS and Linux. In a terminal:
#   curl -fsSL https://github.com/pjlabsdev/hla-wgs/releases/latest/download/install.sh | bash
# Downloads the ready package of the latest release into ~/hla-wgs (or HLA_WGS_HOME), keeps an
# existing reference (ref/) and results, and starts it. Files fetched this way carry no quarantine
# mark, so macOS opens the start file later with a double-click and without a warning.
set -eu
REPO="pjlabsdev/hla-wgs"
DEST="${HLA_WGS_HOME:-$HOME/hla-wgs}"
case "${HLA_WGS_LANG:-${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}}" in de*) DE=1 ;; *) DE=0 ;; esac
[ "$DE" = 0 ] && [ "$(uname -s)" = Darwin ] && [ "$(defaults read -g AppleLocale 2>/dev/null | cut -c1-2)" = de ] && DE=1
say() { if [ "$DE" = 1 ]; then printf '  %s\n' "$1"; else printf '  %s\n' "$2"; fi; }

case "$(uname -s)" in
  Darwin) ASSET="hla-wgs-macos.zip"; START="hla-wgs-start.command" ;;
  Linux)
    case "$(uname -m)" in x86_64|amd64) ASSET="hla-wgs-linux-x86_64.zip"; START="hla-wgs-start.sh" ;;
      *) say "Für $(uname -m) gibt es kein fertiges Paket; bitte aus dem Quelltext bauen (cargo build --release)." "No ready package for $(uname -m); please build from source (cargo build --release)."; exit 1 ;; esac ;;
  *) say "Unter Windows bitte hla-wgs-windows-x86_64.zip von https://github.com/$REPO/releases/latest laden." "On Windows please download hla-wgs-windows-x86_64.zip from https://github.com/$REPO/releases/latest."; exit 1 ;;
esac
for t in curl unzip; do command -v "$t" >/dev/null 2>&1 || { say "$t fehlt." "$t is missing."; exit 1; }; done

printf '\n'
say "hla-wgs wird installiert nach $DEST" "Installing hla-wgs to $DEST"
TMP="$(mktemp -d -t hla-wgs-install.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT
curl -fSL --progress-bar -o "$TMP/$ASSET" "https://github.com/$REPO/releases/latest/download/$ASSET"
unzip -q "$TMP/$ASSET" -d "$TMP/x"
SRC="$TMP/x/hla-wgs"
[ -x "$SRC/hla-wgs" ] || { say "Das Paket enthält kein Programm." "The package holds no program."; exit 1; }
mkdir -p "$DEST"
# copy over an existing installation: ref/ and other files there stay
(cd "$SRC" && tar cf - .) | (cd "$DEST" && tar xf -)
chmod +x "$DEST/hla-wgs" "$DEST/$START"
command -v xattr >/dev/null 2>&1 && xattr -dr com.apple.quarantine "$DEST" 2>/dev/null || true
say "Fertig: $("$DEST/hla-wgs" --version). Später wieder starten mit $DEST/$START." "Done: $("$DEST/hla-wgs" --version). Start it again later with $DEST/$START."
[ "$(uname -s)" = Darwin ] && open "$DEST" 2>/dev/null || true
# start it now; the terminal, not this piped script, is its input
if (: </dev/tty) 2>/dev/null; then HLA_WGS_NO_UPDATE=1 exec bash "$DEST/$START" "$@" </dev/tty; fi
