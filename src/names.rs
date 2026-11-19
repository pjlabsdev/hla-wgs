//! allele-name helpers mirroring the Python prototype
pub fn two(name: &str) -> String {
    // ":".join(name.split(":")[:2])  (keeps the gene prefix)
    name.split(':').take(2).collect::<Vec<_>>().join(":")
}
pub fn three(name: &str) -> String {
    name.split(':').take(3).collect::<Vec<_>>().join(":")
}
/// fields after '*', trailing expression letter removed from the last field
pub fn fields(name: &str) -> Vec<String> {
    let s = name.split('*').nth(1).unwrap_or("");
    let s = strip_letter(s);
    s.split(':').map(|x| x.to_string()).collect()
}
pub fn strip_letter(s: &str) -> &str {
    // re.sub(r"[A-Z]$", "", s)
    match s.as_bytes().last() {
        Some(c) if c.is_ascii_uppercase() => &s[..s.len() - 1],
        _ => s,
    }
}
/// name ends with N/Q/L/S expression suffix
pub fn is_null_like(name: &str) -> bool {
    matches!(name.as_bytes().last(), Some(b'N' | b'Q' | b'L' | b'S'))
}
/// first field (2, 3 or 4) at which two names differ; 9 if they agree on 4 fields
pub fn diff_level(a: &str, b: &str) -> u32 {
    let (f1, f2) = (fields(a), fields(b));
    let sl = |f: &Vec<String>, a: usize, b: usize| f[a.min(f.len())..b.min(f.len())].to_vec();
    if sl(&f1, 0, 2) != sl(&f2, 0, 2) {
        return 2;
    }
    if sl(&f1, 2, 3) != sl(&f2, 2, 3) {
        return 3;
    }
    if sl(&f1, 3, 4) != sl(&f2, 3, 4) {
        return 4;
    }
    9
}
/// sort key: [int(f) if f.isdigit() else 999 for f in fields]
pub fn num_key(name: &str) -> Vec<i64> {
    fields(name).iter().map(|f| if !f.is_empty() && f.bytes().all(|c| c.is_ascii_digit()) { f.parse().unwrap_or(999) } else { 999 }).collect()
}
