#!/bin/bash
# fetch HPRC R2 haplotype assemblies, keep MHC contigs, crop to MHC window -> truth/S.{pat,mat|h1,h2}.mhc.fa
W=${HLA_WORK:-$PWD}; s=$1; cd $W
awk -F, -v s="$s" 'NR==1 { for (i = 1; i <= NF; i++) c[$i] = i; next }
  $c["sample_id"] == s { n = $c["assembly_name"]; h = n ~ /_pat_/ ? "pat" : n ~ /_mat_/ ? "mat" : "h" $c["haplotype"]
    u = $c["assembly"]; sub("^s3://human-pangenomics/", "https://human-pangenomics.s3.amazonaws.com/", u); print h, u }' r2.index.csv | while read h u; do
  [ -s truth/$s.$h.mhc.fa ] && continue
  for i in 1 2 3 4 5; do curl -sS -C - --retry 5 -o dl_$s.$h.fa.gz "$u" && break; sleep 5; done
  gunzip -c dl_$s.$h.fa.gz | hla-wgs-validate mhc-contigs truth/$s.$h.fa 2> truth/$s.$h.log
  rm -f dl_$s.$h.fa.gz
  [ -s truth/$s.$h.fa ] && hla-wgs-validate mhc-crop truth/$s.$h.fa >/dev/null 2>&1 && rm -f truth/$s.$h.fa
done
