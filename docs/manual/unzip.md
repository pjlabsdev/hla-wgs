# Unpacking a ZIP archive

    hla-wgs unzip ZIP DIR

Unpacks a ZIP archive (stored or deflated entries) into `DIR`, so that the start file needs no
`unzip` program on a freshly installed system. Used for `hla.dat.zip`. Every entry's checksum is
verified; an entry that would be written outside `DIR` is refused.

Errors: HW-E040 (damaged), HW-E041 (unsafe path), HW-E042 (compression not supported), see
[error codes](errors.md).
