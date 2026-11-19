//! `PREFIX.html`: the result as one self-contained page for reading and printing. It shows the
//! same values as `PREFIX.json` and is built from it; language by `HLA_WGS_LANG` (de/en) or `LANG`.
use serde_json::Value;

pub const DOI: &str = "10.5281/zenodo.24367470";
pub const ARTICLE: &str = "https://pjlabs.dev/forschung/hla-wgs";
pub const SOURCE: &str = "https://github.com/pjlabsdev/hla-wgs";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn german() -> bool {
    let v = std::env::var("HLA_WGS_LANG").or_else(|_| std::env::var("LC_ALL")).or_else(|_| std::env::var("LANG")).unwrap_or_default();
    v.to_ascii_lowercase().starts_with("de")
}

/// calendar date (UTC) from seconds since 1970
pub fn date(secs: u64) -> (i64, u32, u32) {
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

const CSS: &str = "
:root{--fg:#111;--mut:#6b6b6b;--line:#e3e3e3;--bg:#fff;--acc:#0b57d0;--ok:#1a7f37;--warn:#9a6700;--soft:#f6f6f6}
@media (prefers-color-scheme:dark){:root{--fg:#eaeaea;--mut:#9a9a9a;--line:#2c2c2c;--bg:#111;--acc:#8ab4f8;--ok:#56d364;--warn:#e3b341;--soft:#1a1a1a}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:400 14px/1.55 'IBM Plex Mono',ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
main{max-width:980px;margin:0 auto;padding:40px 24px 56px}
h1{font-size:22px;font-weight:500;margin:0}
h2{font-size:13px;font-weight:500;letter-spacing:.08em;text-transform:uppercase;color:var(--mut);margin:36px 0 10px}
.sub{color:var(--mut);margin-top:4px}
.cards{display:grid;grid-template-columns:repeat(4,1fr);gap:12px;margin-top:24px}
.card{border:1px solid var(--line);border-radius:8px;padding:12px 14px}
.card b{display:block;font-size:20px;font-weight:500}
.card span{color:var(--mut);font-size:12px}
table{width:100%;border-collapse:collapse}
th{font-weight:500;text-align:left;color:var(--mut);font-size:12px;border-bottom:1px solid var(--fg);padding:6px 10px 6px 0}
td{border-bottom:1px solid var(--line);padding:8px 10px 8px 0;vertical-align:top}
td.g{font-weight:500;white-space:nowrap}
.call{font-weight:500;white-space:nowrap}
.full{display:block;color:var(--mut);font-size:12px}
.d{white-space:nowrap;line-height:1.55}
th:nth-child(4),td:nth-child(4){width:70px}
.f{display:inline-block;width:9px;height:9px;border-radius:50%;border:1px solid var(--mut);margin-right:3px}
.f.on{background:var(--ok);border-color:var(--ok)}
.note{color:var(--mut);font-size:12px}
.warn{color:var(--warn)}
.pos{color:var(--warn);font-weight:500}
.cls td{background:var(--soft);color:var(--mut);font-size:12px;letter-spacing:.06em;text-transform:uppercase;padding:5px 8px;border-bottom:1px solid var(--line)}
a{color:var(--acc);text-decoration:none}a:hover{text-decoration:underline}
footer{margin-top:40px;border-top:1px solid var(--fg);padding-top:14px;color:var(--mut);font-size:12px}
footer p{margin:6px 0}
.cite{color:var(--fg)}
@media (max-width:720px){.cards{grid-template-columns:repeat(2,1fr)}.hide{display:none}}
@media print{main{padding:0;max-width:none}a{color:inherit}tr{break-inside:avoid}body{font-size:11px}}
";

fn class_of(g: &str) -> u8 {
    match g {
        "A" | "B" | "C" => 0,
        "E" | "F" | "G" => 2,
        "H" | "J" | "K" | "L" | "V" | "Y" => 3,
        "MICA" | "MICB" | "TAP1" | "TAP2" | "HFE" => 4,
        _ if g.starts_with('D') => 1,
        _ => 4,
    }
}

pub fn render(rep: &Value, sample: &str) -> String {
    let de = german();
    let t = |d: &'static str, e: &'static str| if de { d } else { e };
    let version = rep["version"].as_str().unwrap_or("");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = date(now);
    let day = if de { format!("{d:02}.{m:02}.{y}") } else { format!("{y}-{m:02}-{d:02}") };
    let empty = vec![];
    let genes = rep["genes"].as_array().unwrap_or(&empty);

    let (mut n_alleles, mut n4, mut n_new) = (0, 0, 0);
    let mut rows = String::new();
    let mut last = 9u8;
    let names = [
        t("Klasse I", "Class I"),
        t("Klasse II", "Class II"),
        t("Klasse I, nicht klassisch", "Class I, non-classical"),
        t("Pseudogene", "Pseudogenes"),
        t("Weitere Gene", "Other genes"),
    ];
    let mut order: Vec<&Value> = genes.iter().collect();
    order.sort_by_key(|g| class_of(g["gene"].as_str().unwrap_or("")));
    for g in order {
        let name = g["gene"].as_str().unwrap_or("");
        let c = class_of(name);
        if c != last {
            rows.push_str(&format!("<tr class=\"cls\"><td colspan=\"5\">{}</td></tr>\n", names[c as usize]));
            last = c;
        }
        let copies = g["copies"].as_u64().unwrap_or(2);
        let als = g["alleles"].as_array().unwrap_or(&empty);
        let called = g["called"].as_bool().unwrap_or(false);
        let mut cells = [String::from("<td class=\"note\">–</td>"), String::from("<td class=\"note\">–</td>")];
        let mut dots = String::new();
        let mut notes: Vec<String> = vec![];
        if !called {
            notes.push(t("kein Ergebnis", "no result").to_string());
        } else if copies == 0 {
            notes.push(t("Gen auf keinem Chromosom vorhanden", "gene present on neither chromosome").to_string());
        } else {
            for (i, a) in als.iter().take(copies.min(2) as usize).enumerate() {
                let safe = a["safe_call"].as_str().unwrap_or("");
                let full = a["allele"].as_str().unwrap_or("");
                let res = a["resolution"].as_u64().unwrap_or(0);
                n_alleles += 1;
                if res >= 4 {
                    n4 += 1;
                }
                let extra = if full != safe { format!("<span class=\"full\">{} {}</span>", t("am nächsten:", "nearest:"), esc(full)) } else { String::new() };
                cells[i] = format!("<td><span class=\"call\">{}</span>{extra}</td>", esc(safe));
                dots.push_str(&format!("<div class=\"d\" title=\"{res} {}\">", t("Felder gesichert", "fields supported")));
                for k in 1..=4 {
                    dots.push_str(if k <= res { "<i class=\"f on\"></i>" } else { "<i class=\"f\"></i>" });
                }
                dots.push_str("</div>");
                if let Some(x) = a["expression"].as_str() {
                    notes.push(format!("{}: {}", esc(full), esc(x)));
                }
                let sites = a["new_sites"].as_array().map(|v| v.iter().filter(|s| !s["change"].as_str().unwrap_or("").starts_with("blind")).count()).unwrap_or(0);
                if sites > 0 {
                    n_new += 1;
                    notes.push(format!(
                        "<span class=\"warn\">{} {}: {sites} {}</span>",
                        t("Allel", "allele"),
                        i + 1,
                        t("Abweichung(en) vom Datenbank-Allel, mögliches neues Allel", "difference(s) from the database allele, possibly a new allele")
                    ));
                }
                if let Some(x) = a["renamed_from"].as_str() {
                    notes.push(format!("{} {}", t("mit Varianten benannt, Ausgang", "named with its variants, from"), esc(x)));
                }
            }
            if copies == 1 {
                notes.push(t("eine Kopie", "one copy").to_string());
            } else if als.len() >= 2 && als[0]["safe_call"] == als[1]["safe_call"] {
                notes.push(t("beide Kopien gleich", "both copies alike").to_string());
            }
        }
        rows.push_str(&format!(
            "<tr><td class=\"g\">{}</td>{}{}<td>{dots}</td><td class=\"note hide\">{}</td></tr>\n",
            esc(name),
            cells[0],
            cells[1],
            notes.join("<br>")
        ));
    }

    let mut pgx = String::new();
    let mut carriers = 0;
    for p in rep["pharmacogenetic_alleles"].as_array().unwrap_or(&empty) {
        let n = p["copies"].as_u64().unwrap_or(0);
        let certain = p["certain"].as_bool().unwrap_or(false);
        let state = match (n, certain) {
            (0, true) => t("nicht nachgewiesen", "not found").to_string(),
            (0, false) => format!("<span class=\"warn\">{}</span>", t("nicht sicher beurteilbar", "cannot be judged with certainty")),
            (n, c) => {
                carriers += 1;
                format!("<span class=\"pos\">{} ({n}×){}</span>", t("Träger", "carrier"), if c { "" } else { t(", unsicher", ", uncertain") })
            }
        };
        pgx.push_str(&format!(
            "<tr><td class=\"g\">HLA-{}</td><td>{state}</td><td>{}</td><td class=\"note hide\">{}</td></tr>\n",
            esc(p["allele"].as_str().unwrap_or("")),
            esc(p["drugs"].as_str().unwrap_or("")),
            esc(p["source"].as_str().unwrap_or(""))
        ));
    }

    let mut h = String::with_capacity(32_000);
    h.push_str(&format!(
        "<!DOCTYPE html>\n<html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>hla-wgs · {}</title>\n<style>{CSS}</style></head><body><main>\n",
        t("de", "en"),
        esc(sample)
    ));
    h.push_str(&format!(
        "<h1>{}</h1>\n<div class=\"sub\">{} · hla-wgs {} · {day}</div>\n",
        esc(sample),
        t("HLA-Typisierung aus dem Genom", "HLA typing from the genome"),
        esc(version)
    ));
    h.push_str(&format!(
        "<div class=\"cards\"><div class=\"card\"><b>{}</b><span>{}</span></div><div class=\"card\"><b>{n4} / {n_alleles}</b><span>{}</span></div><div class=\"card\"><b>{n_new}</b><span>{}</span></div><div class=\"card\"><b>{carriers}</b><span>{}</span></div></div>\n",
        genes.len(),
        t("Gene ausgewertet", "genes typed"),
        t("Allele auf vier Felder gesichert", "alleles supported to four fields"),
        t("Allele mit Abweichungen", "alleles with differences"),
        t("pharmakogenetische Treffer", "pharmacogenetic findings")
    ));
    h.push_str(&format!(
        "<h2>{}</h2>\n<table><thead><tr><th>{}</th><th>{} 1</th><th>{} 2</th><th>{}</th><th class=\"hide\">{}</th></tr></thead><tbody>\n{rows}</tbody></table>\n<p class=\"note\">{}</p>\n",
        t("Ergebnis", "Result"),
        t("Gen", "Gene"),
        t("Allel", "Allele"),
        t("Allel", "Allele"),
        t("Felder", "Fields"),
        t("Hinweis", "Note"),
        t(
            "Gezeigt ist der sichere Ruf: so viele Felder des Allelnamens, wie die Reads tragen. Die Punkte zählen diese Felder. Der nächstliegende volle Name steht darunter, wenn er weiter reicht.",
            "Shown is the safe call: as many fields of the allele name as the reads support. The dots count these fields. The nearest full name is given below it where it goes further."
        )
    ));
    if !pgx.is_empty() {
        h.push_str(&format!(
            "<h2>{}</h2>\n<table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th class=\"hide\">{}</th></tr></thead><tbody>\n{pgx}</tbody></table>\n",
            t("Pharmakogenetische Allele", "Pharmacogenetic alleles"),
            t("Allel", "Allele"),
            t("Befund", "Finding"),
            t("Wirkstoffe", "Drugs"),
            t("Quelle", "Source")
        ));
    }
    h.push_str(&format!(
        "<footer>\n<p class=\"cite\">{}: Jungwirth P. hla-wgs: HLA typing from whole-genome sequencing with a per-allele statement of what is certain. Zenodo. <a href=\"https://doi.org/{DOI}\">https://doi.org/{DOI}</a></p>\n<p>{}: <a href=\"{ARTICLE}\">{ARTICLE}</a> · {}: <a href=\"{SOURCE}\">{SOURCE}</a></p>\n<p>{}</p>\n<p>{}</p>\n</footer>\n</main></body></html>\n",
        t("Zitieren", "Cite"),
        t("Forschungsbeitrag mit Methode und Prüfung", "Research article with method and validation"),
        t("Quelltext", "Source code"),
        t(
            "Allelnamen nach IPD-IMGT/HLA. Alle Einzelwerte stehen in den Dateien .conf.tsv und .json neben dieser Seite.",
            "Allele names follow IPD-IMGT/HLA. All values are in the files .conf.tsv and .json next to this page."
        ),
        t(
            "Nur für Forschung (RUO). Kein Medizinprodukt. Vor jeder medizinischen Entscheidung ist eine Bestätigung durch ein akkreditiertes HLA-Labor nötig.",
            "Research use only (RUO). Not a medical device. Any medical decision needs confirmation by an accredited HLA laboratory."
        )
    ));
    h
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates() {
        assert_eq!(super::date(0), (1970, 1, 1));
        assert_eq!(super::date(1_795_046_400), (2026, 11, 19));
        assert_eq!(super::date(951_782_400), (2000, 2, 29));
    }
    #[test]
    fn page_escapes_and_links() {
        let rep = serde_json::json!({"version": "x", "genes": [{"gene": "A", "called": true, "copies": 2, "alleles": [
            {"allele": "A*01:01:01:01", "safe_call": "A*01:01:01", "resolution": 3, "new_sites": []},
            {"allele": "A*02:01:01:01", "safe_call": "A*02:01:01:01", "resolution": 4, "new_sites": [{"change": "A>G"}]}]}],
            "pharmacogenetic_alleles": []});
        let h = super::render(&rep, "a<b");
        assert!(h.contains("a&lt;b") && h.contains(super::DOI) && h.contains(super::ARTICLE));
        assert!(h.contains("1 / 2"));
    }
}
