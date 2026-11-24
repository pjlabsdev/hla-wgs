# Start file

The release packages hold a start file for every system:

| System | Start file |
|---|---|
| macOS | `hla-wgs-start.command` (double-click; opens in Terminal) |
| Linux | `./hla-wgs-start.sh` in a terminal (the same script as `start/hla-wgs-start.command`) |
| Windows | `hla-wgs-start.bat`, which runs `hla-wgs-start.ps1` in PowerShell |

Or in one line on macOS and Linux, installing to `~/hla-wgs`:

    curl -fsSL https://github.com/pjlabsdev/hla-wgs/releases/latest/download/install.sh | bash

## What it does

1. Looks for the `hla-wgs` program next to itself, in `../target/release` or in `PATH`.
2. Checks for a newer release and offers the update.
3. On the first start: downloads `hla.dat` of IPD-IMGT/HLA 3.66, unpacks it with
   [unzip](unzip.md), builds the database with [build-db](build-db.md) in `ref/hladb`, copies the
   repeat mask `data/panel_mask_3.66.bed` and fetches the four WMDA files
   ([database folder](database.md)). Once, a few minutes.
4. Asks for the input: drag the FASTQ R1, BAM or CRAM into the window, or type the path.
5. Types into `~/Downloads/hla-wgs-<sample>` (a new folder if one with results exists), shows the
   progress and the result table, and opens `PREFIX.html`.

## Switches of the start file

| Variable | Effect |
|---|---|
| `HLA_WGS_DB` | use this database folder instead of `ref/hladb` |
| `HLA_WGS_RELEASE` | another IPD-IMGT/HLA release, e.g. `3.67`: built once as `ref/hladb_3.67`, the mask carried over with `build-mask --from` |
| `HLA_WGS_LONG=1` | PacBio HiFi reads ([long](long.md)); the same as starting with `--long` |
| `HLA_WGS_LANG` | `de` or `en` for the start file and the result page |
| `HLA_WGS_NO_UPDATE=1` | no check for a newer release |
| `HLA_WGS_HOME` | installation folder of `install.sh` (default `~/hla-wgs`) |

All switches of the program itself are in [switches](switches.md).

## macOS: first start

The package is not notarized, so macOS refuses the first start. Open System Settings, Privacy &
Security, click "Open Anyway" at the bottom and confirm with your password (once). The one-line
installation does not need this.
