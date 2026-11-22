#!/usr/bin/env python3
"""keep a fraction of the read pairs, chosen by a hash of the read name: thin_pairs.py IN_PREFIX FRAC OUT_PREFIX (reads IN_PREFIX.R1.fq.gz and .R2.fq.gz)"""
import gzip, sys, zlib
src, frac, dst = sys.argv[1], float(sys.argv[2]), sys.argv[3]
for m in ("R1", "R2"):
    with gzip.open(f"{src}.{m}.fq.gz", "rt") as fi, gzip.open(f"{dst}.{m}.fq.gz", "wt", compresslevel=1) as fo:
        while True:
            rec = [fi.readline() for _ in range(4)]
            if not rec[0]:
                break
            name = rec[0].split()[0]
            if name.endswith("/1") or name.endswith("/2"):
                name = name[:-2]
            if (zlib.crc32(name.encode()) % 10000) < frac * 10000:
                fo.write("".join(rec))
