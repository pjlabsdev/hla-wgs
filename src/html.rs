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
:root{--bg:#FAF9F5;--surface:#FFFFFF;--fg:#141413;--mut:#3F3E38;--soft-fg:#6E6C63;--line:#D8D5C7;--hair:rgba(216,213,199,.78);--acc:#D97757;--acc-t:#B34D30;--ok:#248A3D;--okdot:#34C759;--warn:#B34D30;--soft:#F2F0E8;--shadow:0 1px 2px rgba(20,20,19,.04),0 10px 28px rgba(20,20,19,.055);--r:14px;--ease:cubic-bezier(.22,.61,.36,1)}
@media (prefers-color-scheme:dark){:root{--bg:#1C1C1E;--surface:#2C2C2E;--fg:#FFFFFF;--mut:rgba(235,235,245,.72);--soft-fg:rgba(235,235,245,.55);--line:#38383A;--hair:rgba(255,255,255,.1);--acc-t:#E38B69;--ok:#30D158;--okdot:#30D158;--warn:#E38B69;--soft:#242426;--shadow:0 1px 2px rgba(0,0,0,.28),0 12px 30px rgba(0,0,0,.3)}}
@font-face{font-family:'IBM Plex Mono';font-weight:400;font-style:normal;font-display:swap;src:url(data:font/woff2;base64,F400) format('woff2')}
@font-face{font-family:'IBM Plex Mono';font-weight:500;font-style:normal;font-display:swap;src:url(data:font/woff2;base64,F500) format('woff2')}
*{box-sizing:border-box}
html{-webkit-text-size-adjust:100%}
body{margin:0;background:var(--bg);color:var(--fg);font:400 14px/1.6 'IBM Plex Mono',ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;-webkit-font-smoothing:antialiased}
.bar{position:sticky;top:0;z-index:5;background:color-mix(in srgb,var(--bg) 82%,transparent);-webkit-backdrop-filter:saturate(180%) blur(18px);backdrop-filter:saturate(180%) blur(18px);border-bottom:1px solid var(--hair)}
.bar-in{max-width:1000px;margin:0 auto;padding:12px 24px;display:flex;align-items:center;justify-content:space-between;gap:16px}
.brand{font-weight:500;letter-spacing:.02em;font-size:13px}.brand i{font-style:normal;color:var(--acc-t)}
.seg{display:inline-flex;background:var(--soft);border:1px solid var(--hair);border-radius:999px;padding:3px;gap:2px}
.seg button{font:inherit;font-size:12px;font-weight:500;background:none;border:0;border-radius:999px;color:var(--soft-fg);padding:5px 14px;cursor:pointer;transition:background 140ms var(--ease),color 140ms var(--ease)}
.seg button:hover{color:var(--fg)}
body[data-lang=de] nav.lang button[data-set=de],body[data-lang=en] nav.lang button[data-set=en],body[data-tab=r] nav.tabs button[data-tab=r],body[data-tab=d] nav.tabs button[data-tab=d]{background:var(--surface);color:var(--fg);box-shadow:0 1px 2px rgba(20,20,19,.12)}
main{max-width:1000px;margin:0 auto;padding:40px 24px 64px}
body[data-lang=de] main.l-en,body[data-lang=en] main.l-de{display:none}
.eyebrow{font-size:12px;font-weight:500;letter-spacing:.1em;text-transform:uppercase;color:var(--acc-t);margin-bottom:8px}
h1{font-size:30px;line-height:1.2;font-weight:500;margin:0;letter-spacing:-.01em}
h2{font-size:12px;font-weight:500;letter-spacing:.1em;text-transform:uppercase;color:var(--soft-fg);margin:40px 0 12px}
.sub{color:var(--soft-fg);margin-top:8px;font-size:13px}
nav.tabs{margin:28px 0 4px}
.cards{display:grid;grid-template-columns:repeat(4,1fr);gap:14px;margin-top:28px}
.card{background:var(--surface);border:1px solid var(--hair);border-radius:var(--r);padding:18px 18px 16px;box-shadow:var(--shadow)}
.card b{display:block;font-size:26px;line-height:1.15;font-weight:500;letter-spacing:-.01em}
.card span{display:block;margin-top:6px;color:var(--soft-fg);font-size:12px;line-height:1.4}
.panel{background:var(--surface);border:1px solid var(--hair);border-radius:var(--r);box-shadow:var(--shadow);padding:6px 20px;overflow-x:auto}
table{width:100%;border-collapse:collapse}
th{font-weight:500;text-align:left;color:var(--soft-fg);font-size:11px;letter-spacing:.06em;text-transform:uppercase;border-bottom:1px solid var(--line);padding:12px 12px 10px 0}
td{border-bottom:1px solid var(--hair);padding:11px 12px 11px 0;vertical-align:top}
tr:last-child td{border-bottom:0}
td.g{font-weight:500;white-space:nowrap}
.call{font-weight:500;white-space:nowrap}
.full{display:block;color:var(--soft-fg);font-size:12px}
.d{white-space:nowrap;line-height:1.6}
th:nth-child(4),td:nth-child(4){width:76px}
.f{display:inline-block;width:8px;height:8px;border-radius:50%;border:1px solid var(--line);margin-right:4px}
.f.on{background:var(--okdot);border-color:var(--okdot)}
.note{color:var(--soft-fg);font-size:12px;line-height:1.55}
.warn{color:var(--warn)}
.pos{color:var(--warn);font-weight:500}
.cls td{background:none;color:var(--acc-t);font-size:11px;font-weight:500;letter-spacing:.1em;text-transform:uppercase;padding:20px 0 8px;border-bottom:1px solid var(--line)}
a{color:var(--acc-t);text-decoration:none}a:hover{text-decoration:underline}
details{background:var(--surface);border:1px solid var(--hair);border-radius:var(--r);margin:0 0 10px;padding:0 20px;box-shadow:0 1px 2px rgba(20,20,19,.04);transition:box-shadow 220ms var(--ease)}
details[open]{box-shadow:var(--shadow);padding-bottom:12px}
summary{cursor:pointer;list-style:none;padding:14px 0;display:flex;align-items:baseline;gap:12px}summary::-webkit-details-marker{display:none}
summary:after{content:'';margin-left:auto;width:7px;height:7px;border-right:1.5px solid var(--soft-fg);border-bottom:1.5px solid var(--soft-fg);transform:rotate(-45deg);transition:transform 220ms var(--ease);align-self:center}
details[open] summary:after{transform:rotate(45deg)}
summary b{font-weight:500;min-width:4.5em}
.kv{color:var(--mut);font-size:12px;margin:10px 0}
.kv b{color:var(--fg);font-weight:500}
.dt{font-size:12px;margin:6px 0 12px;background:var(--bg);border-radius:10px}
.dt th{padding:9px 12px 7px}.dt td{padding:7px 12px}.dt td:last-child{min-width:16em}
.reason{font-size:12px;color:var(--mut);border-left:2px solid var(--acc);padding:2px 0 2px 10px;margin:4px 0 14px}
.ok{color:var(--ok)}
.alert{margin:22px 0 0;padding:12px 16px;border-radius:var(--r);background:color-mix(in srgb,var(--acc) 12%,var(--surface));border:1px solid color-mix(in srgb,var(--acc) 40%,transparent);font-size:12px;line-height:1.55}.alert.soft{background:var(--surface);border-color:var(--hair)}
@media print{.bar,nav.tabs{display:none}section.tab-r,section.tab-d{display:block!important}section.tab-d{break-before:page}}
body[data-tab=r] section.tab-d,body[data-tab=d] section.tab-r{display:none}
footer{margin-top:48px;border-top:1px solid var(--line);padding-top:18px;color:var(--soft-fg);font-size:12px}
footer p{margin:6px 0}
.cite{color:var(--fg)}
@media (max-width:720px){.cards{grid-template-columns:repeat(2,1fr)}.hide{display:none}h1{font-size:24px}.panel,details{padding-left:14px;padding-right:14px}}
@media print{:root{--bg:#fff;--surface:#fff;--shadow:none}main{padding:0;max-width:none}a{color:inherit}tr,details{break-inside:avoid}body{font-size:11px}.card,.panel,details{box-shadow:none}}
";

// IBM Plex Mono (SIL Open Font License 1.1, src/fonts/OFL.txt), embedded so the page needs no network
const FONT400: &[u8] = include_bytes!("fonts/plex-mono-400.woff2");
const FONT500: &[u8] = include_bytes!("fonts/plex-mono-500.woff2");

fn b64(d: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(d.len() / 3 * 4 + 4);
    for c in d.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            o.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    o
}

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
    let css = CSS.replace("F400", &b64(FONT400)).replace("F500", &b64(FONT500));
    let de = german();
    let first = if de { "de" } else { "en" };
    format!(
        "<!DOCTYPE html>\n<html lang=\"{first}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>hla-wgs · {}</title>\n<style>{css}</style></head><body data-lang=\"{first}\" data-tab=\"r\">\n<header class=\"bar\"><div class=\"bar-in\"><div class=\"brand\">PJ Labs <i>·</i> hla-wgs</div><nav class=\"lang seg\"><button type=\"button\" data-set=\"de\">Deutsch</button><button type=\"button\" data-set=\"en\">English</button></nav></div></header>\n<main lang=\"de\" class=\"l-de\">\n{}</main>\n<main lang=\"en\" class=\"l-en\">\n{}</main>\n<script>\nfunction setL(l){{document.body.dataset.lang=l;document.documentElement.lang=l;try{{localStorage.setItem('hla-wgs-lang',l)}}catch(e){{}}}}\ntry{{const s=localStorage.getItem('hla-wgs-lang');if(s==='de'||s==='en')setL(s)}}catch(e){{}}\ndocument.querySelectorAll('nav.lang button').forEach(b=>b.addEventListener('click',()=>setL(b.dataset.set)));\ndocument.querySelectorAll('nav.tabs button').forEach(b=>b.addEventListener('click',()=>{{document.body.dataset.tab=b.dataset.tab;scrollTo(0,0)}}));\naddEventListener('beforeprint',()=>document.querySelectorAll('details').forEach(d=>d.open=true));\n</script></body></html>\n",
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
        "<div class=\"eyebrow\">{}</div><h1>{}</h1>\n<div class=\"sub\">hla-wgs {} · {day}</div>\n",
        t("HLA-Typisierung aus dem Genom", "HLA typing from the genome"),
        esc(sample),
        esc(version)
    ));
    if rep["repeat_mask"].as_bool() == Some(false) {
        h.push_str(&format!("<div class=\"alert\">{}</div>\n", t(
            "Ohne Wiederholungsmaske gerechnet: Reads aus ähnlichen Stellen des Genoms können mitgezählt werden. Das Ergebnis ist weniger zuverlässig als in der Prüfung (zu viele Felder bei MICA/MICB, scheinbare Abweichungen bei DRB1/DRB4). Maske einrichten (hla-wgs build-mask) und neu starten.",
            "Run without the repeat mask: reads from similar places in the genome can be counted. The result is less reliable than in the validation (too many fields for MICA/MICB, apparent differences in DRB1/DRB4). Set up the mask (hla-wgs build-mask) and run again."
        )));
    }
    if !rep["groups_loaded"].as_bool().unwrap_or(true) {
        h.push_str(&format!("<div class=\"alert soft\">{}</div>\n", t(
            "G- und P-Gruppen fehlen: hla_nom_g.txt und hla_nom_p.txt liegen nicht in der Datenbank.",
            "G and P groups are missing: hla_nom_g.txt and hla_nom_p.txt are not in the database."
        )));
    }
    h.push_str(&format!(
        "<nav class=\"tabs seg\"><button type=\"button\" data-tab=\"r\">{}</button><button type=\"button\" data-tab=\"d\">{}</button></nav>\n<section class=\"tab-r\">\n",
        t("Ergebnis", "Result"),
        t("Details je Gen", "Details per gene")
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
        "<h2>{}</h2>\n<div class=\"panel\"><table><thead><tr><th>{}</th><th>{} 1</th><th>{} 2</th><th>{}</th><th class=\"hide\">{}</th></tr></thead><tbody>\n{rows}</tbody></table></div>\n<p class=\"note\">{}</p>\n",
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
            "<h2>{}</h2>\n<div class=\"panel\"><table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th class=\"hide\">{}</th></tr></thead><tbody>\n{pgx}</tbody></table></div>\n",
            t("Pharmakogenetische Allele", "Pharmacogenetic alleles"),
            t("Allel", "Allele"),
            t("Befund", "Finding"),
            t("Wirkstoffe", "Drugs"),
            t("Quelle", "Source")
        ));
    }
    h.push_str(&format!(
        "</section>\n<section class=\"tab-d\">\n<h2>{}</h2>\n<p class=\"note\">{}</p>\n{details}</section>\n",
        t("Details je Gen", "Details per gene"),
        t(
            "Alle Werte aus .tsv und .conf.tsv. Vorsprung: wie viel besser (Log-Likelihood) das gewählte Allel die Reads erklärt als das beste Allel, das sich in diesem Feld unterscheidet; ab der Schwelle gilt das Feld als gesichert. Ein Klick öffnet ein Gen; beim Drucken sind alle offen.",
            "All values of .tsv and .conf.tsv. Lead: how much better (log-likelihood) the chosen allele explains the reads than the best allele that differs in this field; at or above the threshold the field counts as supported. Click a gene to open it; all are open when printed."
        )
    ));
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
        o.push_str(&format!("<div class=\"reason\">{}: {}</div>\n", t("Grund der Feldzahl", "Why this many fields"), field_reason(name, a, t, thr)));
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
    if !sites.is_empty() && sites.iter().all(|x| x["change"].as_str().unwrap_or("").starts_with("blind")) {
        return t("Strecke ohne eindeutig zuordenbare Reads (ähnliche Gene, Wiederholungen): dort lässt sich das vierte Feld nicht prüfen", "stretch without uniquely placed reads (similar genes, repeats): the fourth field cannot be checked there").to_string();
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
