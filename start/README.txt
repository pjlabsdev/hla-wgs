hla-wgs - HLA typing from whole-genome sequencing
=================================================

(Deutsch weiter unten.)

Research use only. Results must not be used for transplantation, pharmacogenetic or any other
medical decision without confirmation by an accredited HLA laboratory.

Start
-----
macOS    Double-click hla-wgs-start.command. If macOS refuses to open it the first time, right-click
         it, choose Open and confirm once.
Windows  Double-click hla-wgs-start.bat.
Linux    In a terminal: ./hla-wgs-start.sh

Then drag your file (FASTQ R1, BAM or CRAM) into the window, or type its path, and press Enter.

The first start sets up the reference in the folder ref/ next to the start file: it downloads the
IPD-IMGT/HLA database (about 100 MB), builds it and keeps it for later runs (a few minutes, once).
Results are written to Downloads/hla-wgs-SAMPLE: PREFIX.html (result page for reading and
printing), PREFIX.tsv, PREFIX.conf.tsv, PREFIX.json and PREFIX.novel.tsv.

Updates
-------
At every start the start file asks GitHub whether a newer release exists and offers to install it.
The update replaces the program files in this folder; ref/ and your results stay. Set
HLA_WGS_NO_UPDATE=1 to switch the check off.

Language
--------
English, or German on a German system. HLA_WGS_LANG=en or HLA_WGS_LANG=de chooses it.

Requirements
------------
macOS 11 or later (Apple silicon or Intel), Windows 10 or 11 (64 bit), Linux x86_64 with glibc 2.35
or later. About 2 GB of memory and 1 GB of free disk space. An internet connection for the first
start.

More: https://github.com/pjlabsdev/hla-wgs   Citation: see CITATION.cff   Licence: MIT (LICENSE).
The IPD-IMGT/HLA data that the start file downloads is under its own licence (CC BY-ND 4.0).


hla-wgs - HLA-Typisierung aus Ganzgenomsequenzierung
====================================================

Nur für die Forschung. Ergebnisse dürfen ohne Bestätigung durch ein akkreditiertes HLA-Labor nicht
für Transplantation, Pharmakogenetik oder andere medizinische Entscheidungen verwendet werden.

Start
-----
macOS    hla-wgs-start.command doppelklicken. Lehnt macOS das erste Öffnen ab: Rechtsklick,
         Öffnen wählen und einmal bestätigen.
Windows  hla-wgs-start.bat doppelklicken.
Linux    Im Terminal: ./hla-wgs-start.sh

Dann die Datei (FASTQ R1, BAM oder CRAM) ins Fenster ziehen oder ihren Pfad eingeben und Enter
drücken.

Der erste Start richtet die Referenz im Ordner ref/ neben der Startdatei ein: Er lädt die
IPD-IMGT/HLA-Datenbank (etwa 100 MB), baut sie und behält sie für spätere Läufe (einmalig einige
Minuten). Die Ergebnisse liegen in Downloads/hla-wgs-PROBE: PREFIX.html (Ergebnisseite zum Lesen
und Drucken), PREFIX.tsv, PREFIX.conf.tsv, PREFIX.json und PREFIX.novel.tsv.

Updates
-------
Bei jedem Start fragt die Startdatei bei GitHub nach, ob es eine neuere Version gibt, und bietet die
Installation an. Das Update ersetzt die Programmdateien in diesem Ordner; ref/ und Ihre Ergebnisse
bleiben. HLA_WGS_NO_UPDATE=1 schaltet die Prüfung ab.

Sprache
-------
Englisch, auf einem deutschen System Deutsch. HLA_WGS_LANG=de oder HLA_WGS_LANG=en legt sie fest.

Voraussetzungen
---------------
macOS 11 oder neuer (Apple Silicon oder Intel), Windows 10 oder 11 (64 Bit), Linux x86_64 mit
glibc 2.35 oder neuer. Etwa 2 GB Arbeitsspeicher und 1 GB freier Speicherplatz. Beim ersten Start
eine Internetverbindung.

Mehr: https://github.com/pjlabsdev/hla-wgs   Zitieren: siehe CITATION.cff   Lizenz: MIT (LICENSE).
Die IPD-IMGT/HLA-Daten, die die Startdatei lädt, stehen unter ihrer eigenen Lizenz (CC BY-ND 4.0).
