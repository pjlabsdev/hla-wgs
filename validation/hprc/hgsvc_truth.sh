#!/bin/bash
# HGSVC3 verkko assemblies -> truth/S.h{1,2}.mhc.fa and exon tables
W=/tmp/claude-0/hla; cd $W; s=$1
B=https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/HGSVC3/working
for h in 1 2; do
  [ -s truth/$s.h$h.mhc.fa ] && continue
  ok=0
  for b in 20230818_verkko_batch1 20230927_verkko_batch2 20240201_verkko_batch3; do
    u=$B/$b/assemblies/$s/$s.vrk-ps-sseq.asm-hap$h.fasta.gz
    [ "$(curl -sS -I -m 30 -o /dev/null -w '%{http_code}' "$u" 2>/dev/null)" = 200 ] || continue
    for i in 1 2 3; do
      curl -sS --retry 3 -m 3000 "$u" | gunzip -c 2>/dev/null | python3 scripts/mhc_contigs.py truth/$s.h$h.fa 2> truth/$s.h$h.log
      [ -s truth/$s.h$h.fa ] && ok=1 && break; sleep 10
    done
    break
  done
  [ $ok = 1 ] || { echo "$s h$h no_asm" >> lr/hgsvc.fail; continue; }
  [ -s truth/$s.h$h.fa ] && CROPONLY=1 python3 scripts/asm_type2.py truth/$s.h$h.fa >/dev/null 2>&1 && rm -f truth/$s.h$h.fa
done
for f in truth/$s.h1.mhc.fa truth/$s.h2.mhc.fa; do
  [ -s $f ] || continue
  [ -s ${f%.mhc.fa}.exon.tsv ] || nice python3 scripts/truth_exon.py $f > /dev/null 2>&1
  [ -s ${f%.mhc.fa}.exonx.tsv ] || nice python3 scripts/truth_exonx.py $f > /dev/null 2>&1
  [ -s ${f%.mhc.fa}.exony.tsv ] || nice python3 scripts/truth_exony.py $f > /dev/null 2>&1
done
