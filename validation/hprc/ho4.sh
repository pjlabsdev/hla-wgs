#!/bin/bash
# samples not used for the four-field rules: assemblies -> full-gene truth inputs, reads -> hla-wgs 1.0.0
cd ${HLA_WORK:-$PWD}; s=$1; B=hla-wgs
[ -s newg_ho/$s.json ] && ls truth/$s.*.exony.tsv >/dev/null 2>&1 && exit 0
bash scripts/fetch_r2.sh $s
for f in truth/$s.*.mhc.fa; do
  [ -s $f ] || { echo "$s no_asm" >> newg/ho4.fail; exit 0; }
  [ -s ${f%.mhc.fa}.exonx.tsv ] || nice python3 scripts/truth_exonx.py $f > /dev/null 2>&1
  [ -s ${f%.mhc.fa}.exony.tsv ] || nice python3 scripts/truth_exony.py $f > /dev/null 2>&1
done
if [ ! -s newg_ho/$s.json ]; then
  for t in 1 2; do scripts/extract_sr.sh $s && break; sleep 20; done
  rm -f sr/$s.mhc.bam sr/$s.un.bam
  [ -s sr/$s.R1.fq.gz ] || { echo "$s extract_fail" >> newg/ho4.fail; exit 0; }
  nice $B bdtest sr/$s.R1.fq.gz sr/$s.R2.fq.gz newg_ho/$s 2 > newg_ho/$s.log 2>&1
  rm -f sr/$s.R1.fq.gz sr/$s.R2.fq.gz
fi
exit 0
