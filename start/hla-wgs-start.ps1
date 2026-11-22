# hla-wgs Start for Windows: double-click hla-wgs-start.bat (or run this file in PowerShell).
# Asks for the input file; drag the file into the window (the path is inserted) or type the path.
# Checks for a new release, sets up the IPD-IMGT/HLA reference once if it is missing, runs hla-wgs
# with a progress display and shows the result. Long reads (PacBio HiFi): HLA_WGS_LONG=1 or -Long.
param([switch]$Long)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 } catch {}

$BaseRel = '3.66'
$Repo = 'pjlabsdev/hla-wgs'
$FreqUrl = "https://raw.githubusercontent.com/$Repo/main/data/freq_1000g_2field.json"
$MaskUrl = "https://raw.githubusercontent.com/$Repo/main/data/panel_mask_3.66.bed"
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path

# language: HLA_WGS_LANG=de|en wins, otherwise German on a German system, otherwise English
if ($env:HLA_WGS_LANG -like 'de*') { $DE = $true }
elseif ($env:HLA_WGS_LANG -like 'en*') { $DE = $false }
else { $DE = (Get-Culture).TwoLetterISOLanguageName -eq 'de' }
function T([string]$de, [string]$en) { if ($DE) { $de } else { $en } }

# ── display (PJ Labs: gold accent) ───────────────────────────────────────────
$W = 64
function Line([string]$left, [string]$right, [string]$color = 'DarkYellow') {
  $gap = [Math]::Max(1, $W - 2 - $left.Length - $right.Length)
  Write-Host ('  ' + $left) -NoNewline -ForegroundColor $color
  Write-Host ((' ' * $gap) + $right) -ForegroundColor DarkGray
}
function Divider { Write-Host ('  ' + ([string][char]0x2500) * ($W - 2)) -ForegroundColor DarkGray }
function Ok([string]$label, [string]$detail, [string]$time) {
  $l = if ($detail) { "$label  $detail" } else { $label }
  $gap = [Math]::Max(1, $W - 6 - $l.Length - $time.Length)
  Write-Host '    ' -NoNewline; Write-Host ([char]0x2713) -NoNewline -ForegroundColor Green
  Write-Host " $label" -NoNewline
  if ($detail) { Write-Host "  $detail" -NoNewline -ForegroundColor DarkGray }
  Write-Host ((' ' * $gap) + $time) -ForegroundColor DarkGray
}
function Fail([string]$label, [string]$detail) {
  Write-Host '    ' -NoNewline; Write-Host ([char]0x2717 + " $label  $detail") -ForegroundColor Red
}
function Secs($t0) { '{0:N1}s' -f ((Get-Date) - $t0).TotalSeconds }
function Finish([int]$code = 0) {
  Write-Host ''
  Read-Host (T '  Enter zum Schließen' '  Press Enter to close') | Out-Null
  exit $code
}
function Ask([string]$de, [string]$en) {
  Write-Host ''
  Write-Host ('  ' + (T $de $en))
  Write-Host '  > ' -NoNewline -ForegroundColor DarkYellow
  return [Console]::ReadLine()
}
function CleanPath([string]$p) {
  if ($null -eq $p) { return '' }
  $p = $p.Trim()
  if ($p.Length -ge 2 -and (($p.StartsWith('"') -and $p.EndsWith('"')) -or ($p.StartsWith("'") -and $p.EndsWith("'")))) { $p = $p.Substring(1, $p.Length - 2) }
  if ($p.StartsWith('& ')) { $p = CleanPath $p.Substring(2) }
  return $p
}
function AskPath([string]$de, [string]$en, [switch]$AllowEmpty) {
  while ($true) {
    $p = CleanPath (Ask $de $en)
    if ($p -eq '' -and $AllowEmpty) { return '' }
    if ($p -ne '' -and (Test-Path -LiteralPath $p -PathType Leaf)) { return (Resolve-Path -LiteralPath $p).Path }
    Write-Host ('  ' + (T 'Datei nicht gefunden:' 'File not found:') + " $p") -ForegroundColor Red
  }
}
# runs a program with a spinner; returns its exit code (output goes to $WorkLog)
$Frames = @([char]0x280B, [char]0x2819, [char]0x2839, [char]0x2838, [char]0x283C, [char]0x2834, [char]0x2826, [char]0x2827, [char]0x2807, [char]0x280F)
function Quote([string]$a) { if ($a -match '[\s"]') { '"' + ($a -replace '"', '\"') + '"' } else { $a } }
function Step([string]$label, [string]$exe, [string[]]$argv, [string]$log) {
  $p = Start-Process -FilePath $exe -ArgumentList (($argv | ForEach-Object { Quote $_ }) -join ' ') -NoNewWindow -PassThru -RedirectStandardOutput $log -RedirectStandardError "$log.err"
  $i = 0
  while (-not $p.HasExited) {
    Write-Host ("`r    " + $Frames[$i % 10] + " $label   ") -NoNewline -ForegroundColor DarkYellow
    Start-Sleep -Milliseconds 100; $i++
  }
  $p.WaitForExit()
  Write-Host ("`r" + (' ' * ($W + 4)) + "`r") -NoNewline
  if (Test-Path "$log.err") { Get-Content "$log.err" | Add-Content $log; Remove-Item "$log.err" -ErrorAction SilentlyContinue }
  return $p.ExitCode
}
function Download([string]$url, [string]$target) {
  try { Invoke-WebRequest -Uri $url -OutFile $target -UseBasicParsing; return $true } catch { return $false }
}

Clear-Host
$Host.UI.RawUI.WindowTitle = 'hla-wgs'
Write-Host ''
Line 'hla-wgs' (Get-Date -Format 'dd.MM.yyyy  HH:mm')
Write-Host ('  ' + (T 'HLA-Typisierung aus dem Genom · nur für Forschung (RUO)' 'HLA typing from the genome · research use only (RUO)')) -ForegroundColor DarkGray
Divider

$Bin = Join-Path $Here 'hla-wgs.exe'
if (-not (Test-Path $Bin)) {
  Write-Host ''
  Write-Host ('  ' + (T 'Das Programm hla-wgs.exe wurde neben dieser Datei nicht gefunden.' 'The hla-wgs.exe program was not found next to this file.')) -ForegroundColor Red
  Finish 1
}
Unblock-File -LiteralPath $Bin -ErrorAction SilentlyContinue
$WorkLog = Join-Path $env:TEMP ('hla-wgs-start-' + [Guid]::NewGuid().ToString('N') + '.log')

# ── update: on every start ask GitHub for the latest release and offer it (HLA_WGS_NO_UPDATE=1 skips) ──
function VerNum([string]$v) {
  $v = ($v -replace '^v', '') -replace '[-+].*$', ''
  $p = $v.Split('.') + @('0', '0', '0')
  return [int64]$p[0] * 100000000 + [int64]$p[1] * 10000 + [int64]$p[2]
}
function Newer([string]$latest, [string]$cur) {
  $a = VerNum $latest; $b = VerNum $cur
  if ($a -gt $b) { return $true }
  return ($a -eq $b -and $cur.Contains('-') -and -not $latest.Contains('-'))
}
if ($env:HLA_WGS_NO_UPDATE -ne '1') {
  try {
    $cur = ((& $Bin --version) -split '\s+')[1]
    $rel = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -TimeoutSec 5 -Headers @{ Accept = 'application/vnd.github+json' }
    $latest = ($rel.tag_name -replace '^v', '')
    if ($latest -and $cur -and (Newer $latest $cur)) {
      Write-Host ''
      Line (T 'Neue Version verfügbar' 'New version available') "$cur -> $latest"
      $yn = Ask 'Jetzt aktualisieren? [j/N]' 'Update now? [y/N]'
      if ($yn -match '^[jJyY]') {
        $tmp = Join-Path $env:TEMP ('hla-wgs-update-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $tmp | Out-Null
        $zip = Join-Path $tmp 'hla-wgs-windows-x86_64.zip'
        $t0 = Get-Date
        Write-Host ''
        if (Download "https://github.com/$Repo/releases/download/v$latest/hla-wgs-windows-x86_64.zip" $zip) {
          Ok 'Download' 'hla-wgs-windows-x86_64.zip' (Secs $t0)
          $t0 = Get-Date
          Expand-Archive -LiteralPath $zip -DestinationPath (Join-Path $tmp 'x') -Force
          $exe = Get-ChildItem -Path (Join-Path $tmp 'x') -Recurse -Filter 'hla-wgs.exe' | Select-Object -First 1
          if ($exe) {
            # everything of the package replaces its copy here; ref\ and other files stay
            Get-ChildItem -LiteralPath $exe.DirectoryName | Copy-Item -Destination $Here -Recurse -Force
            Get-ChildItem -LiteralPath $Here -Recurse | Unblock-File -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
            Ok 'Update' $latest (Secs $t0)
            Write-Host ('  ' + (T 'Startet neu …' 'Restarting …')) -ForegroundColor DarkGray
            Start-Sleep -Seconds 1
            $env:HLA_WGS_NO_UPDATE = '1'
            $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', (Join-Path $Here 'hla-wgs-start.ps1'))
            if ($Long) { $a += '-Long' }
            & powershell.exe @a
            exit $LASTEXITCODE
          } else { Fail 'Update' (T 'Programm fehlt im Paket' 'program missing in the package') }
        } else { Fail 'Download' 'hla-wgs-windows-x86_64.zip' }
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
      }
    }
  } catch { }
}

# ── reference: HLA_WGS_DB, else ref\hladb next to this file (built there on the first start) ──
$Rel = if ($env:HLA_WGS_RELEASE) { $env:HLA_WGS_RELEASE } else { $BaseRel }
if ($Rel -notmatch '^\d\.\d\d(\.\d)?$') {
  Write-Host ('  ' + (T 'HLA_WGS_RELEASE muss wie 3.67 aussehen:' 'HLA_WGS_RELEASE must look like 3.67:') + " $Rel") -ForegroundColor Red
  Finish 1
}
$Rel = ($Rel.Split('.')[0..1]) -join '.'
$BaseDb = Join-Path $Here 'ref\hladb'
$Db = if ($env:HLA_WGS_DB) { $env:HLA_WGS_DB } elseif ($Rel -eq $BaseRel) { $BaseDb } else { Join-Path $Here "ref\hladb_$Rel" }
function RelBranch([string]$r) { $p = $r.Split('.'); '{0}{1:D2}0' -f [int]$p[0], [int]$p[1] }
function DbFormat([string]$d) { $f = Join-Path $d 'format'; if (Test-Path $f) { [int](Get-Content $f -Raw).Trim() } else { 1 } }
$FmtNeed = 1
try { $FmtNeed = [int]((& $Bin --db-format) | Select-Object -First 1) } catch {}

function BuildRef([string]$db, [string]$rel, [bool]$ask) {
  $work = Split-Path -Parent $db
  New-Item -ItemType Directory -Path $db -Force | Out-Null
  Write-Host "  $db" -ForegroundColor DarkGray
  $dat = ''
  if ($ask) {
    $dat = AskPath 'hla.dat hineinziehen oder Pfad eingeben, oder nur Enter zum Herunterladen (etwa 45 MB):' 'Drop hla.dat or enter its path, or just press Enter to download it (about 45 MB):' -AllowEmpty
  }
  Write-Host ''
  $t0 = Get-Date
  $own = $false
  if ($dat -eq '') {
    $zip = Join-Path $work 'hla.dat.zip'
    Write-Host ('    Download  IPD-IMGT/HLA ' + $rel + ' …') -NoNewline -ForegroundColor DarkYellow
    $okd = Download "https://github.com/ANHIG/IMGTHLA/raw/$(RelBranch $rel)/hla.dat.zip" $zip
    Write-Host ("`r" + (' ' * ($W + 4)) + "`r") -NoNewline
    if (-not $okd) { Fail 'Download' "IPD-IMGT/HLA $rel"; Finish 1 }
    Ok 'Download' ('{0:N1} MB' -f ((Get-Item $zip).Length / 1MB)) (Secs $t0)
    $t0 = Get-Date
    Expand-Archive -LiteralPath $zip -DestinationPath $work -Force
    Ok (T 'Entpacken' 'Unpacking') 'hla.dat' (Secs $t0)
    $dat = Join-Path $work 'hla.dat'
    $own = $true
  }
  $freq = Join-Path $Here 'freq_1000g_2field.json'
  if (-not (Test-Path $freq)) {
    $freq = Join-Path $work 'freq_1000g_2field.json'
    if (-not (Test-Path $freq) -and -not (Download $FreqUrl $freq)) { Fail (T 'Häufigkeitstabelle' 'Frequency table') ''; Finish 1 }
  }
  $t0 = Get-Date
  $rc = Step (T 'Datenbank bauen' 'Building the database') $Bin @('build-db', $dat, $freq, $db) $WorkLog
  if ($rc -ne 0) { Fail (T 'Datenbank' 'Database') ''; Get-Content $WorkLog | Write-Host; Finish 1 }
  Ok (T 'Datenbank' 'Database') 'build-db' (Secs $t0)
  if ($own) { Remove-Item -LiteralPath $dat, (Join-Path $work 'hla.dat.zip') -ErrorAction SilentlyContinue }
}
function ShippedMask([string]$db) {
  $m = Join-Path $db 'panel.fa.mask.bed'
  $c = Join-Path $Here 'panel_mask_3.66.bed'
  if (Test-Path $c) { Copy-Item -LiteralPath $c -Destination $m -Force } else { Download $MaskUrl $m | Out-Null }
  return ((Test-Path $m) -and (Get-Item $m).Length -gt 0)
}

$dbOld = (Test-Path (Join-Path $Db 'hladb.json.gz')) -and ((DbFormat $Db) -lt $FmtNeed)
if (-not (Test-Path (Join-Path $Db 'hladb.json.gz')) -or -not (Test-Path (Join-Path $Db 'panel.fa')) -or $dbOld) {
  Write-Host ''
  if ($dbOld) { Line (T 'Referenz wird erneuert' 'Reference is renewed') "IPD-IMGT/HLA $Rel" } else { Line (T 'Einmalige Einrichtung' 'One-time setup') "IPD-IMGT/HLA $Rel" }
  if ($Rel -ne $BaseRel) { Remove-Item -LiteralPath (Join-Path $Db 'panel.fa.mask.bed') -ErrorAction SilentlyContinue }
  BuildRef $Db $Rel ($Rel -eq $BaseRel)
}
$wmda = "https://github.com/ANHIG/IMGTHLA/raw/$(RelBranch $Rel)/wmda"
foreach ($pair in @(@('hla_nom_g.txt', 'hla_nom_p.txt', (T 'G- und P-Gruppen' 'G and P groups')), @('rel_dna_ser.txt', 'rel_ser_ser.txt', (T 'Serologische Antigene' 'Serological antigens')))) {
  $a = Join-Path $Db $pair[0]; $b = Join-Path $Db $pair[1]
  if (-not (Test-Path $a) -or -not (Test-Path $b)) {
    $t0 = Get-Date
    if ((Download "$wmda/$($pair[0])" $a) -and (Download "$wmda/$($pair[1])" $b)) { Ok $pair[2] $pair[0] (Secs $t0) }
    else { Remove-Item -LiteralPath $a, $b -ErrorAction SilentlyContinue; Fail $pair[2] (T 'weiter ohne' 'continuing without') }
  }
}
$mask = Join-Path $Db 'panel.fa.mask.bed'
if (-not (Test-Path $mask)) {
  $t0 = Get-Date
  if ($Rel -eq $BaseRel) {
    if (ShippedMask $Db) { Ok (T 'Wiederholungsmaske' 'Repeat mask') "IMGT $BaseRel" (Secs $t0) }
  } else {
    if (-not (Test-Path (Join-Path $BaseDb 'panel.fa'))) {
      Line (T 'Vorlage für die Wiederholungsmaske' 'Template for the repeat mask') "IPD-IMGT/HLA $BaseRel"
      BuildRef $BaseDb $BaseRel $false
    }
    if (-not (Test-Path (Join-Path $BaseDb 'panel.fa.mask.bed'))) { ShippedMask $BaseDb | Out-Null }
    $rc = Step (T 'Wiederholungsmaske übertragen' 'Carrying the repeat mask over') $Bin @('build-mask', $Db, '--from', $BaseDb) $WorkLog
    if ($rc -eq 0 -and (Test-Path $mask)) { Ok (T 'Wiederholungsmaske' 'Repeat mask') ((T 'übertragen von' 'carried over from') + " $BaseRel") (Secs $t0) }
    else { Remove-Item -LiteralPath $mask -ErrorAction SilentlyContinue }
  }
}
if (-not (Test-Path $mask)) {
  Write-Host ('  ' + (T 'Ohne Wiederholungsmaske ist das Ergebnis weniger zuverlässig.' 'Without the repeat mask the result is less reliable.')) -ForegroundColor Red
  $fa = AskPath 'Wiederholungsmaske (einmalig): Genom GRCh38 als FASTA hineinziehen, oder Enter zum Überspringen:' 'Repeat mask (once): drop the GRCh38 genome FASTA, or press Enter to skip:' -AllowEmpty
  if ($fa -ne '') {
    $t0 = Get-Date
    $rc = Step (T 'Wiederholungsmaske bauen' 'Building the repeat mask') $Bin @('build-mask', $Db, $fa) $WorkLog
    if ($rc -eq 0) { Ok (T 'Wiederholungsmaske' 'Repeat mask') 'build-mask' (Secs $t0) } else { Fail (T 'Wiederholungsmaske' 'Repeat mask') (T 'weiter ohne Maske' 'continuing without') }
  }
}

# ── input ─────────────────────────────────────────────────────────────────────
function Stop([string]$msg) { Write-Host "  $msg" -ForegroundColor Red; Finish 1 }
$In = AskPath 'BAM, CRAM oder FASTQ (R1) hier hineinziehen oder Pfad eingeben, dann Enter:' 'Drop a BAM, CRAM or FASTQ (R1) here or enter its path, then press Enter:'
$lower = $In.ToLowerInvariant()
$extra = @(); $In2 = ''
$isLong = $Long -or $env:HLA_WGS_LONG -eq '1'
$isFq = $lower -match '\.(fq|fastq)(\.gz)?$'
if ($lower.EndsWith('.bam')) {
  if (-not ((Test-Path "$In.bai") -or (Test-Path ([IO.Path]::ChangeExtension($In, '.bai'))) -or (Test-Path "$In.csi"))) { Stop (T 'Zur BAM fehlt der Index (.bai daneben).' 'The BAM index (.bai) is missing.') }
} elseif ($lower.EndsWith('.cram')) {
  if (-not (Test-Path "$In.crai")) { Stop (T 'Zur CRAM fehlt der Index (.crai daneben).' 'The CRAM index (.crai) is missing.') }
} elseif (-not $isFq) {
  Stop (T 'Unbekanntes Format. Erlaubt sind BAM, CRAM oder FASTQ.' 'Unknown format. Allowed are BAM, CRAM or FASTQ.')
}
if (-not $isLong) {
  if ($lower.EndsWith('.cram')) {
    $extra = @(AskPath 'Referenz der CRAM (GRCh38-FASTA mit .fai) hineinziehen:' 'Drop the reference of the CRAM (GRCh38 FASTA with .fai):')
  } elseif ($isFq) {
    $In2 = AskPath 'Zweite FASTQ-Datei (R2) hineinziehen:' 'Drop the second FASTQ file (R2):'
    Write-Host ('  ' + (T 'FASTQ: Das Programm liest alle Reads des Genoms, nicht nur die HLA-Region wie bei einer BAM. Das dauert deutlich länger. Weitere Programme braucht es nicht.' 'FASTQ: the program reads every read of the genome, not only the HLA region as with a BAM. This takes much longer. No other programs are needed.')) -ForegroundColor DarkGray
  }
}

$S = [IO.Path]::GetFileName($In)
foreach ($x in @('.fastq.gz', '.fq.gz', '.fastq', '.fq', '.bam', '.cram')) { if ($S.ToLowerInvariant().EndsWith($x)) { $S = $S.Substring(0, $S.Length - $x.Length); break } }
if ($isFq) { $S = $S -replace '_S\d+_L\d{3}_R1_001$', '' -replace '_L\d{3}_R1_001$', '' -replace '_R1_001$', '' -replace '([._])[Rr]?1$', '' }
$S = $S -replace '[^A-Za-z0-9._-]', '_'   # spaces or quotes in a file name break nothing
if ($S -eq '') { $S = 'sample' }
$Downloads = Join-Path $env:USERPROFILE 'Downloads'
$Out = Join-Path $Downloads "hla-wgs-$S"; $n = 2
while ((Test-Path (Join-Path $Out "$S.tsv")) -or (Test-Path (Join-Path $Out "$S.long.tsv"))) { $Out = Join-Path $Downloads "hla-wgs-$S-$n"; $n++ }
$Threads = [Environment]::ProcessorCount

Write-Host ''
Divider
Line (T 'Probe' 'Sample') $S 'Gray'
Line (T 'Eingabe' 'Input') ([IO.Path]::GetFileName($In)) 'Gray'
Line (T 'Ausgabe' 'Output') $Out 'Gray'
Line 'Threads' ("$Threads" + $(if ($isLong) { '  ·  HiFi' } else { '' })) 'Gray'
Divider
Ask 'Start mit Enter (oder Strg-C zum Abbrechen)' 'Press Enter to start (or Ctrl-C to cancel)' | Out-Null
New-Item -ItemType Directory -Path $Out -Force | Out-Null
$Log = Join-Path $Out "$S.log"
$Prefix = Join-Path $Out $S
$Host.UI.RawUI.WindowTitle = "hla-wgs · $S"
if ($DE) { $env:HLA_WGS_LANG = 'de' }   # result page PREFIX.html in German

if ($isLong) { $argv = @('long', $Db, $In, $Prefix, "$Threads"); $Tab = "$Prefix.long.tsv" }
elseif ($In2 -ne '') { $argv = @($Db, $In, $In2, $Prefix, "$Threads"); $Tab = "$Prefix.conf.tsv" }
else { $argv = @($Db, $In, $Prefix, "$Threads") + $extra; $Tab = "$Prefix.conf.tsv" }

Write-Host ''
$start = Get-Date
$p = Start-Process -FilePath $Bin -ArgumentList (($argv | ForEach-Object { Quote $_ }) -join ' ') -NoNewWindow -PassThru -RedirectStandardOutput "$Log.out" -RedirectStandardError $Log
$i = 0; $phase = 1; $t0 = $start
$lRead = T 'Reads lesen' 'Reading reads'; $lType = T 'Typisierung' 'Typing'
while (-not $p.HasExited) {
  $txt = if (Test-Path $Log) { Get-Content -LiteralPath $Log -Raw -ErrorAction SilentlyContinue } else { '' }
  if ($phase -eq 1 -and $txt -match '(?m)^\[input\]|panel mapped reads') {
    Write-Host ("`r" + (' ' * ($W + 4)) + "`r") -NoNewline
    $m = [regex]::Match($txt, ' (\d+) pairs')
    Ok (T 'Reads gelesen' 'Reads read') $(if ($m.Success) { $m.Groups[1].Value + (T ' Paare' ' pairs') } else { '' }) (Secs $t0)
    $phase = 2; $t0 = Get-Date
  }
  $label = if ($phase -eq 1) { $lRead } else { $lType }
  $gene = ''
  if ($phase -eq 2 -and $txt) { $mm = [regex]::Matches($txt, '\] (\S+) done'); if ($mm.Count -gt 0) { $gene = '  ' + $mm.Count + (T ' Gene' ' genes') } }
  Write-Host ("`r    " + $Frames[$i % 10] + " $label$gene   ") -NoNewline -ForegroundColor DarkYellow
  Start-Sleep -Milliseconds 120; $i++
}
$p.WaitForExit()
Write-Host ("`r" + (' ' * ($W + 4)) + "`r") -NoNewline
if (Test-Path "$Log.out") { Get-Content "$Log.out" | Add-Content $Log; Remove-Item "$Log.out" -ErrorAction SilentlyContinue }
if ($p.ExitCode -ne 0 -or -not (Test-Path $Tab)) {
  Fail (T 'Abgebrochen' 'Stopped') ''
  Get-Content -LiteralPath $Log -Tail 15 | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
  Finish 1
}
Ok $lType '' (Secs $t0)
Write-Host ''
Line (T 'Fertig' 'Done') (Secs $start) 'Green'
Write-Host ''
$rows = Import-Csv -LiteralPath $Tab -Delimiter "`t"
if ($rows) { $rows | Select-Object -Property ($rows[0].PSObject.Properties.Name | Select-Object -First 5) | Format-Table -AutoSize | Out-String -Width 200 | Write-Host }
Write-Host ('  ' + (T 'safe_call und resolution: so viele Felder, wie die Reads tragen. Maßgeblich ist der sichere Ruf.' 'safe_call and resolution: as many fields as the reads support. Use the safe call.')) -ForegroundColor DarkGray
Write-Host "  $Out" -ForegroundColor DarkGray
$Host.UI.RawUI.WindowTitle = "hla-wgs · $S · " + (T 'fertig' 'done')
Invoke-Item -LiteralPath $Out
if (Test-Path "$Prefix.html") { Invoke-Item -LiteralPath "$Prefix.html" }
Finish 0
