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
details{border-bottom:1px solid var(--line);padding:6px 0}
summary{cursor:pointer;list-style:none}summary::-webkit-details-marker{display:none}
summary:before{content:'+ ';color:var(--mut)}details[open] summary:before{content:'– '}
.kv{color:var(--mut);font-size:12px;margin:8px 0}
.dt{font-size:12px;margin:6px 0 10px}.dt td,.dt th{padding:4px 10px 4px 0}
.ok{color:var(--ok)}
body[data-lang=de] main.l-en,body[data-lang=en] main.l-de{display:none}
nav.lang{max-width:980px;margin:0 auto;padding:16px 24px 0;text-align:right}
nav.lang button{font:inherit;font-size:12px;background:none;border:1px solid var(--line);color:var(--mut);padding:3px 10px;cursor:pointer}
body[data-lang=de] nav.lang button[data-set=de],body[data-lang=en] nav.lang button[data-set=en]{color:var(--fg);border-color:var(--fg)}
@media print{nav.lang{display:none}}
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

/// the page holds the result in German and in English; a switch at the top shows one of them,
/// the default follows HLA_WGS_LANG / LANG
pub fn render(rep: &Value, sample: &str) -> String {
    let de = german();
    let first = if de { "de" } else { "en" };
    format!(
        "<!DOCTYPE html>\n<html lang=\"{first}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>hla-wgs · {}</title>\n<style>{CSS}</style></head><body data-lang=\"{first}\">\n<nav class=\"lang\"><button type=\"button\" data-set=\"de\">Deutsch</button><button type=\"button\" data-set=\"en\">English</button></nav>\n<main lang=\"de\" class=\"l-de\">\n{}</main>\n<main lang=\"en\" class=\"l-en\">\n{}</main>\n<script>\nfunction setL(l){{document.body.dataset.lang=l;document.documentElement.lang=l;try{{localStorage.setItem('hla-wgs-lang',l)}}catch(e){{}}}}\ntry{{const s=localStorage.getItem('hla-wgs-lang');if(s==='de'||s==='en')setL(s)}}catch(e){{}}\ndocument.querySelectorAll('nav.lang button').forEach(b=>b.addEventListener('click',()=>setL(b.dataset.set)));\naddEventListener('beforeprint',()=>document.querySelectorAll('details').forEach(d=>d.open=true));\n</script></body></html>\n",
        esc(sample),
        render_one(rep, sample, true),
        render_one(rep, sample, false)
    )
}

fn render_one(rep: &Value, sample: &str, de: bool) -> String {
    let t = |d: &'static str, e: &'static str| if de { d } else { e };
    let version = rep["version"].as_str().unwrap_or("");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d) = date(now);
    let day = if de { format!("{d:02}.{m:02}.{y}") } else { format!("{y}-{m:02}-{d:02}") };
    let empty = vec![];
    let genes = rep["genes"].as_array().unwrap_or(&empty);

    let (mut n_alleles, mut n4, mut n_new) = (0, 0, 0);
    let mut details = String::new();
    let thr = |k: &str| rep["thresholds"][k].as_f64().unwrap_or(0.0);
    let f1 = |v: &Value| v.as_f64().map(|x| format!("{x:.1}")).unwrap_or_else(|| "–".to_string());
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
        if copies == 0 && g.get("copies").is_some() {
            notes.push(t("Gen auf keinem Chromosom vorhanden", "gene present on neither chromosome").to_string());
        } else if !called {
            notes.push(t("kein Ergebnis", "no result").to_string());
        } else {
            // sites: a site flagged "either" could belong to either copy; it is one finding for the
            // gene, not one per allele. Blind stretches are reported once, with their length.
            let sites_of = |a: &Value| -> Vec<(i64, String, bool)> {
                a["new_sites"].as_array().map(|v| v.iter().map(|x| (x["pos"].as_i64().unwrap_or(-1), x["change"].as_str().unwrap_or("").to_string(), x["either_allele"].as_bool().unwrap_or(false))).collect()).unwrap_or_default()
            };
            let used: Vec<&Value> = als.iter().take(copies.min(2) as usize).collect();
            let either: std::collections::BTreeSet<i64> = used.iter().flat_map(|a| sites_of(a)).filter(|x| x.2).map(|x| x.0).collect();
            let blind = used.iter().flat_map(|a| sites_of(a)).filter_map(|x| x.1.strip_prefix("blind").and_then(|b| b.trim_end_matches("bp").parse::<u64>().ok())).max();
            let mut gene_new = false;
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
                let sites = sites_of(a).iter().filter(|x| !x.1.starts_with("blind") && !either.contains(&x.0)).count();
                if sites > 0 {
                    gene_new = true;
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
            if !either.is_empty() {
                gene_new = true;
                notes.push(format!(
                    "<span class=\"warn\">{} {}</span>",
                    either.len(),
                    t("abweichende Stelle(n), keiner der beiden Kopien sicher zuzuordnen", "differing site(s) that cannot be assigned to either copy")
                ));
            }
            if gene_new {
                n_new += 1;
            }
            if let Some(b) = blind {
                notes.push(format!(
                    "{b} {}",
                    t("bp ohne eindeutig zuordenbare Reads (ähnliche Gene, Wiederholungen): dort ist das Allel nicht prüfbar", "bp without uniquely placed reads (similar genes, repeats): the allele cannot be checked there")
                ));
            }
            if copies == 1 {
                notes.push(t("eine Kopie", "one copy").to_string());
            } else if als.len() >= 2 && als[0]["allele"] == als[1]["allele"] {
                notes.push(t("beide Kopien gleich", "both copies alike").to_string());
            } else if als.len() >= 2 && als[0]["safe_call"] == als[1]["safe_call"] {
                notes.push(t("beide Kopien auf den gesicherten Feldern gleich", "both copies alike on the supported fields").to_string());
            }
        }
        details.push_str(&gene_details(g, &t, &thr, &f1));
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
        "{}",
        ""
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
        t("Gene mit Abweichungen von der Datenbank", "genes with differences from the database"),
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
    h.push_str(&format!(
        "<h2>{}</h2>\n<p class=\"note\">{}</p>\n{details}",
        t("Details je Gen", "Details per gene"),
        t(
            "Alle Werte aus .tsv und .conf.tsv. Vorsprung: wie viel besser (Log-Likelihood) das gewählte Allel die Reads erklärt als das beste Allel, das sich in diesem Feld unterscheidet; ab der Schwelle gilt das Feld als gesichert. Ein Klick öffnet ein Gen; beim Drucken sind alle offen.",
            "All values of .tsv and .conf.tsv. Lead: how much better (log-likelihood) the chosen allele explains the reads than the best allele that differs in this field; at or above the threshold the field counts as supported. Click a gene to open it; all are open when printed."
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
        "<footer>\n<p class=\"cite\">{}: Jungwirth P. hla-wgs: HLA typing from whole-genome sequencing with a per-allele statement of what is certain. Zenodo. <a href=\"https://doi.org/{DOI}\">https://doi.org/{DOI}</a></p>\n<p>{}: <a href=\"{ARTICLE}\">{ARTICLE}</a> · {}: <a href=\"{SOURCE}\">{SOURCE}</a></p>\n<p>{}</p>\n<p>{}</p>\n</footer>\n",
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

/// one collapsible block with every value the program reports for a gene
fn gene_details(g: &Value, t: &dyn Fn(&'static str, &'static str) -> &'static str, thr: &dyn Fn(&str) -> f64, f1: &dyn Fn(&Value) -> String) -> String {
    let name = g["gene"].as_str().unwrap_or("");
    let empty = vec![];
    let als = g["alleles"].as_array().unwrap_or(&empty);
    let copies = g["copies"].as_u64().unwrap_or(2);
    let calls: Vec<String> = als.iter().take(copies.min(2) as usize).map(|a| esc(a["safe_call"].as_str().unwrap_or(""))).collect();
    let summary = if copies == 0 { t("nicht vorhanden", "absent").to_string() } else if !g["called"].as_bool().unwrap_or(false) { t("kein Ergebnis", "no result").to_string() } else { calls.join(" · ") };
    let mut o = format!("<details><summary><b>{}</b> <span class=\"note\">{summary}</span></summary>\n", esc(name));
    o.push_str(&format!(
        "<div class=\"kv\">{}: {} · {}: {} · {}: {}{}</div>\n",
        t("Reads", "Reads"),
        g["reads"].as_u64().map(|x| x.to_string()).unwrap_or_else(|| "–".into()),
        t("Kopien", "Copies"),
        copies,
        t("Vorsprung des besten Allelpaars", "Lead of the best allele pair"),
        f1(&g["gap"]),
        g["copy_ratio"].as_f64().map(|r| format!(" · {}: {r:.2}", t("Kopienverhältnis (1 Kopie ≈ 1, 2 Kopien ≈ 2)", "copy ratio (1 copy ≈ 1, 2 copies ≈ 2)"))).unwrap_or_default()
    ));
    if copies == 0 {
        o.push_str(&format!("<div class=\"kv\">{}</div></details>\n", t("Zu wenige Reads für eine Kopie: das Gen fehlt auf beiden Chromosomen.", "Too few reads for one copy: the gene is absent on both chromosomes.")));
        return o;
    }
    for (i, a) in als.iter().take(copies.min(2) as usize).enumerate() {
        let full = a["allele"].as_str().unwrap_or("");
        o.push_str(&format!(
            "<div class=\"kv\">{} {}: <b>{}</b> · {}: {} · {}: {} · G: {} · P: {}{}{}</div>\n",
            t("Allel", "Allele"),
            i + 1,
            esc(full),
            t("sicherer Ruf", "safe call"),
            esc(a["safe_call"].as_str().unwrap_or("")),
            t("gesicherte Felder", "supported fields"),
            a["resolution"].as_u64().unwrap_or(0),
            esc(a["g_group"].as_str().unwrap_or("–")),
            esc(a["p_group"].as_str().unwrap_or("–")),
            a["expression"].as_str().map(|x| format!(" · {}", esc(x))).unwrap_or_default(),
            a["renamed_from"].as_str().map(|x| format!(" · {} {}", t("benannt aus", "named from"), esc(x))).unwrap_or_default()
        ));
        o.push_str(&format!("<table class=\"dt\"><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>", t("Feld", "Field"), t("Vorsprung", "Lead"), t("Schwelle", "Threshold"), t("nächster Konkurrent", "nearest competitor")));
        for k in ["2", "3", "4"] {
            let m = &a["margin"][k];
            let th = thr(k);
            let pass = m.as_f64().map(|x| x >= th).unwrap_or(true);
            o.push_str(&format!(
                "<tr><td>{k}</td><td class=\"{}\">{}</td><td>{th:.0}</td><td>{}</td></tr>",
                if pass { "ok" } else { "warn" },
                if m.is_null() { t("kein Konkurrent", "no competitor").to_string() } else { f1(m) },
                esc(a["alternative"][k].as_str().filter(|x| !x.is_empty()).unwrap_or("–"))
            ));
        }
        o.push_str("</table>\n");
        o.push_str(&format!("<div class=\"kv\">{}: {}</div>\n", t("Grund der Feldzahl", "Why this many fields"), field_reason(name, a, t, thr)));
        let sites = a["new_sites"].as_array().cloned().unwrap_or_default();
        if !sites.is_empty() {
            o.push_str(&format!("<table class=\"dt\"><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>", t("Position im Gen", "Position in the gene"), t("Abweichung", "Difference"), t("Reads dafür / Tiefe", "Reads for it / depth"), t("Hinweis", "Note")));
            for x in &sites {
                let ch = x["change"].as_str().unwrap_or("");
                let mut nt = vec![];
                if x["exon"].as_bool().unwrap_or(false) { nt.push(t("im Exon", "in an exon")); }
                if x["either_allele"].as_bool().unwrap_or(false) { nt.push(t("keiner Kopie sicher zuzuordnen", "cannot be assigned to one copy")); }
                if ch.starts_with("blind") { nt.push(t("Strecke ohne eindeutig zuordenbare Reads", "stretch without uniquely placed reads")); }
                if ch.contains("unsupported") { nt.push(t("Datenbank-Base von keinem Read gestützt", "database base backed by no read")); }
                let rd = if ch.starts_with("blind") { "–".to_string() } else { format!("{} / {}", x["reads"], x["depth"]) };
                o.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{rd}</td><td>{}</td></tr>", x["pos"], esc(ch), nt.join(", ")));
            }
            o.push_str("</table>\n");
        }
    }
    o.push_str("</details>\n");
    o
}

/// why the safe call stops at its number of fields
fn field_reason(gene: &str, a: &Value, t: &dyn Fn(&'static str, &'static str) -> &'static str, thr: &dyn Fn(&str) -> f64) -> String {
    let r = a["resolution"].as_u64().unwrap_or(0);
    let nf = a["allele"].as_str().and_then(|x| x.split('*').nth(1)).map(|x| x.split(':').count() as u64).unwrap_or(4);
    if r >= 4 || r >= nf {
        return t("alle Felder des Allelnamens gesichert", "all fields of the allele name are supported").to_string();
    }
    let next = (r + 1).max(2).to_string();
    if let Some(m) = a["margin"][next.as_str()].as_f64() {
        if m < thr(&next) {
            return format!("{} {next}: {m:.1} < {:.0}", t("Vorsprung vor dem nächsten Konkurrenten unter der Schwelle im Feld", "lead over the nearest competitor below the threshold at field"), thr(&next));
        }
    }
    if gene == "E" && r == 3 {
        return t("HLA-E wird in dieser Version höchstens auf 3 Felder angegeben", "HLA-E is reported to at most 3 fields in this version").to_string();
    }
    let sites = a["new_sites"].as_array().cloned().unwrap_or_default();
    if sites.iter().any(|x| x["exon"].as_bool().unwrap_or(false)) {
        return t("abweichende Stelle im Exon: das Allel ist neu oder unbekannt, nur die Allelgruppe ist sicher", "differing site in an exon: the allele is new or unknown, only the allele group is certain").to_string();
    }
    if !sites.is_empty() {
        return t("abweichende Stelle außerhalb der Exons: die ersten drei Felder gelten, das vierte nicht", "differing site outside the exons: the first three fields hold, the fourth does not").to_string();
    }
    t("Datenlage reicht nicht für mehr Felder", "the data do not support more fields").to_string()
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
