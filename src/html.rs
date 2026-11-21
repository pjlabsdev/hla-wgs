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
:root{--bg:#FAF9F5;--surface:#FFFFFF;--fg:#141413;--mut:#3F3E38;--soft-fg:#6E6C63;--line:#D8D5C7;--hair:rgba(216,213,199,.78);--acc:#D97757;--acc-t:#B34D30;--ok:#248A3D;--okdot:#34C759;--warn:#B34D30;--soft:#F2F0E8;--f1:#C2410C;--f2:#9A7400;--f3:#4D7C0F;--f4:#0E7490;--shadow:0 1px 2px rgba(20,20,19,.04),0 10px 28px rgba(20,20,19,.055);--r:14px;--ease:cubic-bezier(.22,.61,.36,1)}
@media (prefers-color-scheme:dark){:root{--bg:#1C1C1E;--surface:#2C2C2E;--fg:#FFFFFF;--mut:rgba(235,235,245,.72);--soft-fg:rgba(235,235,245,.55);--line:#38383A;--hair:rgba(255,255,255,.1);--acc-t:#E38B69;--ok:#30D158;--okdot:#30D158;--warn:#E38B69;--soft:#242426;--f1:#FB8A5C;--f2:#E3B341;--f3:#9BCB5A;--f4:#4FC3D9;--shadow:0 1px 2px rgba(0,0,0,.28),0 12px 30px rgba(0,0,0,.3)}}
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
table.tf th:nth-child(4),table.tf td:nth-child(4){width:96px}
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
.nm{white-space:nowrap;font-weight:500;letter-spacing:.01em}.nm .gn{color:var(--soft-fg);font-weight:400}
.fd{-webkit-print-color-adjust:exact;print-color-adjust:exact}
.fd.k1{color:var(--f1)}.fd.k2{color:var(--f2)}.fd.k3{color:var(--f3)}.fd.k4{color:var(--f4)}
.fd.off{color:var(--soft-fg);font-weight:400}
.c{color:var(--soft-fg)}
svg.fdia{-webkit-print-color-adjust:exact;print-color-adjust:exact;display:block;max-width:100%;height:auto;margin:6px 0 10px;overflow:visible}
svg.fdia .fn{font:500 18px 'IBM Plex Mono',ui-monospace,monospace;fill:var(--fg)}svg.fdia .fg{fill:var(--soft-fg);font-weight:400}
svg.fdia .fs{fill:var(--soft-fg)}svg.fdia .off{fill:var(--soft-fg);font-weight:400}
svg.fdia .br,svg.fdia .ln{fill:none;stroke-width:1.6;stroke-linejoin:round;stroke-linecap:round}
svg.fdia .k1{stroke:var(--f1);fill:var(--f1)}svg.fdia .k2{stroke:var(--f2);fill:var(--f2)}svg.fdia .k3{stroke:var(--f3);fill:var(--f3)}svg.fdia .k4{stroke:var(--f4);fill:var(--f4)}
svg.fdia tspan.fk1{fill:var(--f1)}svg.fdia tspan.fk2{fill:var(--f2)}svg.fdia tspan.fk3{fill:var(--f3)}svg.fdia tspan.fk4{fill:var(--f4)}svg.fdia tspan.off{fill:var(--soft-fg)}
svg.fdia .br.off,svg.fdia .ln.off{stroke:var(--line);stroke-dasharray:3 3}svg.fdia circle.off{fill:var(--line);stroke:none}svg.fdia circle{stroke:none}
svg.fdia .br,svg.fdia .ln{fill:none!important}
svg.fdia .lb{font:400 12px 'IBM Plex Mono',ui-monospace,monospace;fill:var(--mut)}svg.fdia .lb.off{fill:var(--soft-fg)}
.legend{background:var(--surface);border:1px solid var(--hair);border-radius:var(--r);box-shadow:var(--shadow);padding:16px 20px 6px;margin:0 0 14px;overflow-x:auto}.legend .note{margin:2px 0 10px}h4.ah{font-size:12px;font-weight:500;letter-spacing:.1em;text-transform:uppercase;color:var(--acc-t);margin:18px 0 2px;padding-top:12px;border-top:1px solid var(--hair)}details h4.ah:first-of-type{border-top:0;padding-top:0}.legend h3{font-size:12px;font-weight:500;letter-spacing:.06em;text-transform:uppercase;color:var(--soft-fg);margin:0 0 6px}
.alert{margin:22px 0 0;padding:12px 16px;border-radius:var(--r);background:color-mix(in srgb,var(--acc) 12%,var(--surface));border:1px solid color-mix(in srgb,var(--acc) 40%,transparent);font-size:12px;line-height:1.55}.alert.soft{background:var(--surface);border-color:var(--hair)}
@media print{.bar,nav.tabs{display:none}section.tab-r,section.tab-d{display:block!important}section.tab-d{break-before:page}}
body[data-tab=r] section.tab-d,body[data-tab=d] section.tab-r{display:none}
.ruo{font-size:11px;font-weight:500;letter-spacing:.06em;text-transform:uppercase;color:var(--acc-t);border:1px solid color-mix(in srgb,var(--acc) 40%,transparent);border-radius:999px;display:inline-block;padding:3px 12px;margin:0 0 22px}
.head{display:grid;grid-template-columns:repeat(4,1fr);gap:0;margin-top:22px;background:var(--surface);border:1px solid var(--hair);border-radius:var(--r);box-shadow:var(--shadow);overflow:hidden}
.head div{padding:12px 16px;border-right:1px solid var(--hair);border-bottom:1px solid var(--hair);min-width:0}.head div:nth-child(4n){border-right:0}.head div:nth-last-child(-n+4){border-bottom:0}
.head span{display:block;font-size:11px;letter-spacing:.06em;text-transform:uppercase;color:var(--soft-fg)}.head b{display:block;font-weight:500;font-size:13px;margin-top:3px;overflow-wrap:anywhere}
table.clin td{padding-top:12px;padding-bottom:12px}table.clin td:nth-child(2),table.clin td:nth-child(3){width:40%}table.clin .call{font-size:15px}
.rw{font-size:11px;color:var(--soft-fg);margin-left:2px}
.gl{display:flex;align-items:center;gap:14px;padding:14px 20px}.gl code{flex:1;font:400 12px/1.6 'IBM Plex Mono',ui-monospace,monospace;overflow-wrap:anywhere;color:var(--mut)}
.copy{font:inherit;font-size:12px;font-weight:500;background:var(--soft);border:1px solid var(--hair);border-radius:999px;padding:5px 14px;color:var(--fg);cursor:pointer}
details.more{margin-top:14px}details.more table{margin-bottom:6px}
.prose{padding:8px 20px}.prose p{margin:10px 0;color:var(--mut);font-size:13px;line-height:1.65}
@page{size:A4;margin:16mm 14mm 18mm;@bottom-right{content:counter(page) ' / ' counter(pages);font:400 9px 'IBM Plex Mono',monospace;color:#6E6C63}@bottom-left{content:'hla-wgs · RUO';font:400 9px 'IBM Plex Mono',monospace;color:#6E6C63}}
@media print{.copy{display:none}.head{grid-template-columns:repeat(4,1fr)}}
@media (max-width:720px){.head{grid-template-columns:repeat(2,1fr)}.head div:nth-child(4n){border-right:1px solid var(--hair)}.head div:nth-child(2n){border-right:0}.head div:nth-last-child(-n+4){border-bottom:1px solid var(--hair)}.head div:nth-last-child(-n+2){border-bottom:0}}
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

/// an allele name with each field marked: supported fields as colour chips by field number, the rest
/// grey; the gene prefix plain
fn name_html(full: &str, res: u64) -> String {
    let (gene, rest) = full.split_once('*').unwrap_or(("", full));
    let mut o = format!("<span class=\"nm\"><span class=\"gn\">{}*</span>", esc(gene));
    for (i, f) in rest.split(':').enumerate() {
        if i > 0 {
            o.push_str(&format!("<span class=\"c{}\">:</span>", if (i as u64) < res { "" } else { " off" }));
        }
        let k = i + 1;
        o.push_str(&format!("<span class=\"fd k{k}{}\" title=\"{k}\">{}</span>", if (k as u64) <= res { "" } else { " off" }, esc(f)));
    }
    o.push_str("</span>");
    o
}

/// what each field of an allele name stands for, drawn on an example name
fn legend(t: &dyn Fn(&'static str, &'static str) -> &'static str, example: &(String, u64, Value), thr: &dyn Fn(&str) -> f64) -> String {
    let labels: Vec<String> = (1..=4u64).map(|k| {
        if k <= example.1 {
            format!("{} · {}", field_meaning(t, k as usize), t("✓ gesichert", "✓ supported"))
        } else if k == example.1 + 1 {
            format!("{} · {}: {}", field_meaning(t, k as usize), t("? nicht gesichert", "? not supported"), short_reason(&example.2, t, thr))
        } else {
            format!("{} · {}", field_meaning(t, k as usize), t("? nicht gesichert", "? not supported"))
        }
    }).collect();
    format!("<div class=\"legend\"><h3>{}</h3>{}<p class=\"note\">{}</p></div>\n", t("So liest man einen Allelnamen", "How to read an allele name"), field_diagram(&example.0, example.1, &labels), t(
        "Beispiel aus dieser Probe. ✓ gesichert: die Reads belegen das Feld. ? nicht gesichert: das Feld gehört zum nächstliegenden Datenbank-Allel, die Reads können es nicht prüfen; der Grund steht dabei. In der Tabelle sind gesicherte Felder farbig, nicht gesicherte grau.",
        "Example from this sample. ✓ supported: the reads prove the field. ? not supported: the field belongs to the nearest database allele and the reads cannot check it; the reason is given. In the table supported fields are coloured, unsupported ones grey."))
}

/// the reason why a field is not supported, in a few words (the full text is in the details)
fn short_reason(a: &Value, t: &dyn Fn(&'static str, &'static str) -> &'static str, thr: &dyn Fn(&str) -> f64) -> &'static str {
    let r = a["resolution"].as_u64().unwrap_or(0);
    let next = (r + 1).max(2).to_string();
    if a["gene_cap"].as_bool().unwrap_or(false) {
        return t("bei diesem Gen in der Validierung nicht verlässlich genug", "not reliable enough for this gene in validation");
    }
    if a["margin"][next.as_str()].as_f64().map(|m| m < thr(&next)).unwrap_or(false) {
        return t("Reads unterscheiden nicht klar vom nächsten Allel", "reads do not clearly tell it from the next allele");
    }
    let sites = a["new_sites"].as_array().cloned().unwrap_or_default();
    if !sites.is_empty() && sites.iter().all(|x| x["change"].as_str().unwrap_or("").starts_with("blind")) {
        return t("Strecke ohne eindeutig zuordenbare Reads", "stretch without uniquely placed reads");
    }
    if sites.iter().any(|x| x["exon"].as_bool().unwrap_or(false)) {
        return t("abweichende Stelle im Exon", "differing site in an exon");
    }
    if !sites.is_empty() {
        return t("abweichende Stelle außerhalb der Exons", "differing site outside the exons");
    }
    t("Datenlage reicht nicht", "not enough evidence")
}

fn field_meaning(t: &dyn Fn(&'static str, &'static str) -> &'static str, k: usize) -> &'static str {
    match k {
        1 => t("Feld 1 · Allelgruppe, etwa das Antigen", "Field 1 · allele group, about the antigen"),
        2 => t("Feld 2 · Protein: andere Aminosäuren", "Field 2 · protein: other amino acids"),
        3 => t("Feld 3 · stille Änderung in den Exons", "Field 3 · silent change in the exons"),
        _ => t("Feld 4 · Änderung außerhalb der Exons", "Field 4 · change outside the exons"),
    }
}

/// an allele name drawn with a bracket under each field and a line to its label, as in the
/// nomenclature figure; `labels[k]` is the text for field k+1, fields beyond `res` are drawn grey
fn field_diagram(full: &str, res: u64, labels: &[String]) -> String {
    let (gene, rest) = full.split_once('*').unwrap_or(("", full));
    let fields: Vec<&str> = rest.split(':').collect();
    let nf = fields.len().min(4);
    let cw = 10.8; // IBM Plex Mono at 18 px
    let lw = 7.2; // at 12 px
    let x0 = 4.0;
    let mut spans = vec![];
    let mut pos = gene.chars().count() + 1;
    for (i, f) in fields.iter().enumerate().take(nf) {
        if i > 0 {
            pos += 1;
        }
        let n = f.chars().count();
        spans.push((x0 + pos as f64 * cw, x0 + (pos + n) as f64 * cw));
        pos += n;
    }
    let name_end = x0 + full.chars().count() as f64 * cw;
    let lx = name_end + 34.0;
    let row = 24.0;
    let top = 50.0;
    let maxl = labels.iter().take(nf).map(|l| l.chars().count()).max().unwrap_or(0) as f64;
    let w = lx + maxl * lw + 8.0;
    let h = top + (nf as f64 - 1.0) * row + 14.0;
    let mut o = format!("<svg class=\"fdia\" viewBox=\"0 0 {w:.0} {h:.0}\" width=\"{w:.0}\" role=\"img\" aria-label=\"{}\">", esc(full));
    o.push_str(&format!("<text x=\"{x0}\" y=\"20\" class=\"fn\"><tspan class=\"fg\">{}*</tspan>", esc(gene)));
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            o.push_str(&format!("<tspan class=\"fs{}\">:</tspan>", if (i as u64) < res { "" } else { " off" }));
        }
        o.push_str(&format!("<tspan class=\"fk{}{}\">{}</tspan>", (i + 1).min(4), if ((i + 1) as u64) <= res { "" } else { " off" }, esc(f)));
    }
    o.push_str("</text>");
    for (i, &(a, b)) in spans.iter().enumerate() {
        let k = i + 1;
        let off = if (k as u64) <= res { "" } else { " off" };
        let y = top + (nf - k) as f64 * row;
        let cx = (a + b) / 2.0;
        o.push_str(&format!(
            "<path class=\"br k{k}{off}\" d=\"M{:.1} 27 v5 H{:.1} v-5\"/><path class=\"ln k{k}{off}\" d=\"M{cx:.1} 32 V{y:.1} H{:.1}\"/><circle class=\"dt k{k}{off}\" cx=\"{:.1}\" cy=\"{y:.1}\" r=\"2.6\"/><text class=\"lb{off}\" x=\"{lx:.1}\" y=\"{:.1}\">{}</text>",
            a + 1.0, b - 1.0, lx - 8.0, lx - 8.0, y + 4.0, esc(&labels[i])
        ));
    }
    o.push_str("</svg>");
    o
}

/// the loci of a clinical HLA typing (transplantation, platelet and donor matching), in report order
const CLINICAL: [&str; 11] = ["A", "B", "C", "DRB1", "DRB3", "DRB4", "DRB5", "DQA1", "DQB1", "DPA1", "DPB1"];

/// resolution in the words of an HLA laboratory
fn res_word(res: u64, t: &dyn Fn(&'static str, &'static str) -> &'static str) -> &'static str {
    match res {
        0 => t("nicht bestimmt", "not determined"),
        1 => t("Allelgruppe (1 Feld)", "allele group (1 field)"),
        2 => t("Protein (2 Felder)", "protein (2 fields)"),
        3 => t("kodierende Sequenz (3 Felder)", "coding sequence (3 fields)"),
        _ => t("Allel (4 Felder)", "allele (4 fields)"),
    }
}

/// a supported call cut to at most two fields, the form clinics work with
fn two_field(safe: &str) -> String {
    let (g, rest) = safe.split_once('*').unwrap_or((safe, ""));
    if rest.is_empty() {
        return format!("HLA-{g}");
    }
    let f: Vec<&str> = rest.split(':').take(2).collect();
    format!("HLA-{g}*{}", f.join(":"))
}

/// short report number from the result itself: the same result gives the same number
fn report_id(rep: &Value) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in rep["genes"].to_string().bytes().chain(rep["input"].to_string().bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("HLA-{:08X}", (h >> 32) as u32)
}

/// Genotype List String (GL String, the exchange format of typing results): "+" joins the two copies
/// of a gene, "^" the genes; only supported fields are written
fn gl_string(genes: &[Value]) -> String {
    let mut loci = vec![];
    for name in CLINICAL {
        let Some(g) = genes.iter().find(|g| g["gene"] == name) else { continue };
        if !g["called"].as_bool().unwrap_or(false) || g["copies"].as_u64() == Some(0) {
            continue;
        }
        let copies = g["copies"].as_u64().unwrap_or(2).min(2) as usize;
        let als: Vec<String> = g["alleles"].as_array().map(|v| v.iter().take(copies).filter_map(|a| a["safe_call"].as_str().filter(|s| s.contains('*')).map(|s| format!("HLA-{s}"))).collect()).unwrap_or_default();
        if !als.is_empty() {
            loci.push(als.join("+"));
        }
    }
    loci.join("^")
}

/// what a new allele differs in, from the annotated sites: count, nearest allele, effect on the protein
fn new_allele_note(a: &Value, i: usize, either: &std::collections::BTreeSet<i64>, t: &dyn Fn(&'static str, &'static str) -> &'static str) -> Option<String> {
    let sites: Vec<&Value> = a["new_sites"].as_array().map(|v| v.iter().filter(|x| {
        let c = x["change"].as_str().unwrap_or("");
        !c.starts_with("blind") && !either.contains(&x["pos"].as_i64().unwrap_or(-1))
    }).collect()).unwrap_or_default();
    if sites.is_empty() {
        return None;
    }
    let eff: Vec<&str> = sites.iter().map(|x| x["effect"].as_str().unwrap_or("-")).collect();
    let coding = eff.iter().filter(|e| !matches!(**e, "-" | "non-coding" | "synonymous") && !e.starts_with("synonymous")).count();
    let impact = if coding > 0 {
        format!("{coding} {}", t("davon mit Folge für das Protein", "of them changing the protein"))
    } else if eff.iter().all(|e| *e != "-") {
        t("Protein unverändert", "protein unchanged").to_string()
    } else {
        t("Wirkung siehe Details", "effect in the details").to_string()
    };
    Some(format!(
        "<span class=\"warn\">{} {}: {} · {} {} {} · {impact}</span>",
        t("Allel", "Allele"),
        i + 1,
        t("neues Allel", "new allele"),
        sites.len(),
        t(if sites.len() == 1 { "Abweichung zu" } else { "Abweichungen zu" }, if sites.len() == 1 { "difference from" } else { "differences from" }),
        esc(a["allele"].as_str().unwrap_or(""))
    ))
}

/// CPIC recommendation for a carrier, in one sentence, with the guideline
fn pgx_rec(allele: &str, t: &dyn Fn(&'static str, &'static str) -> &'static str) -> (&'static str, &'static str, &'static str) {
    match allele {
        "B*57:01" => (t("Abacavir nicht anwenden (Überempfindlichkeitsreaktion).", "Do not use abacavir (hypersensitivity reaction)."), "CPIC 2014", "https://pmc.ncbi.nlm.nih.gov/articles/PMC3994233"),
        "B*15:02" => (t("Carbamazepin und Oxcarbazepin bei Erstgabe nicht anwenden, Phenytoin und Fosphenytoin nicht neu beginnen (SJS/TEN).", "Do not start carbamazepine or oxcarbazepine, do not start phenytoin or fosphenytoin (SJS/TEN)."), "CPIC 2017, 2020", "https://pmc.ncbi.nlm.nih.gov/articles/PMC5847474"),
        "A*31:01" => (t("Carbamazepin bei Erstgabe möglichst durch eine Alternative ersetzen; ohne Alternative engmaschig überwachen (SJS/TEN, DRESS).", "Prefer an alternative to carbamazepine when starting; without one, monitor closely (SJS/TEN, DRESS)."), "CPIC 2017", "https://pmc.ncbi.nlm.nih.gov/articles/PMC5847474"),
        "B*58:01" => (t("Allopurinol nicht anwenden (SJS/TEN, DRESS).", "Do not use allopurinol (SJS/TEN, DRESS)."), "CPIC 2015", "https://pmc.ncbi.nlm.nih.gov/articles/PMC4675696"),
        _ => ("", "CPIC", "https://cpicpgx.org/guidelines/"),
    }
}

/// the clinical summary: the eleven loci at two fields with group, antigen and receptor ligand
fn clinical(genes: &[Value], t: &dyn Fn(&'static str, &'static str) -> &'static str) -> String {
    let empty = vec![];
    let mut rows = String::new();
    let drb345 = genes.iter().any(|g| matches!(g["gene"].as_str(), Some("DRB3" | "DRB4" | "DRB5")) && g["called"].as_bool().unwrap_or(false) && g["copies"].as_u64().unwrap_or(2) > 0);
    for name in CLINICAL {
        if name == "DRB3" && !drb345 {
            rows.push_str(&format!("<tr><td class=\"g\">HLA-DRB3/4/5</td><td class=\"note\" colspan=\"2\">{}</td></tr>\n", t("keines vorhanden", "none present")));
        }
        let Some(g) = genes.iter().find(|g| g["gene"] == name) else { continue };
        let copies = g["copies"].as_u64().unwrap_or(2);
        let present = g["called"].as_bool().unwrap_or(false) && copies > 0;
        if matches!(name, "DRB3" | "DRB4" | "DRB5") {
            if !present {
                continue;
            }
        }
        let als = g["alleles"].as_array().unwrap_or(&empty);
        let mut cells = vec![];
        for k in 0..2usize {
            if !present || k >= copies as usize {
                cells.push(format!("<td class=\"note\">{}</td>", if !present && k == 0 { t("nicht vorhanden", "absent") } else if copies == 1 && k == 1 { t("– (eine Kopie)", "– (one copy)") } else { "–" }));
                continue;
            }
            let Some(a) = als.get(k) else { cells.push("<td>–</td>".into()); continue };
            let res = a["resolution"].as_u64().unwrap_or(0);
            let safe = a["safe_call"].as_str().unwrap_or("");
            let mut sub: Vec<String> = vec![];
            if res >= 2 {
                if let Some(x) = a["g_group"].as_str() { sub.push(format!("G {}", esc(x.split('*').nth(1).unwrap_or(x)))); }
            }
            if let Some(x) = a["serology"].as_str() {
                let assumed = a["serology_assumed"].as_bool().unwrap_or(false);
                sub.push(format!("<span title=\"{}\">{}{}</span>", t("serologisches Antigen (WHO)", "serological antigen (WHO)"), esc(x), if assumed { "*" } else { "" }));
            }
            if let Some(x) = a["ligand"].as_str() { sub.push(format!("<span title=\"{}\">{}</span>", t("Ligand der NK-Zell-Rezeptoren (KIR)", "ligand of the natural killer cell receptors (KIR)"), esc(x))); }
            let expr = a["expression"].as_str().map(|_| {
                let last = a["allele"].as_str().unwrap_or("").chars().last().unwrap_or(' ');
                format!(" <span class=\"pos\" title=\"{}\">{}</span>", t("Expressionsvariante", "expression variant"), match last { 'N' => t("Null-Allel", "null allele"), 'L' => t("geringe Expression", "low expression"), 'S' => t("nur sezerniert", "secreted only"), 'Q' => t("Expression fraglich", "questionable expression"), _ => t("abweichende Expression", "aberrant expression") })
            }).unwrap_or_default();
            let call = if res == 0 { format!("<span class=\"note\">{}</span>", t("nicht bestimmt", "not determined")) } else { format!("<span class=\"call\">{}</span>", esc(&two_field(safe))) };
            cells.push(format!("<td>{call}{expr}<span class=\"full\">{}</span><span class=\"full\">{}</span></td>", sub.join(" · "), res_word(res, t)));
        }
        rows.push_str(&format!("<tr><td class=\"g\">HLA-{}</td>{}</tr>\n", esc(name), cells.join("")));
    }
    format!(
        "<div class=\"panel\"><table class=\"clin\"><thead><tr><th>{}</th><th>{} 1</th><th>{} 2</th></tr></thead><tbody>\n{rows}</tbody></table></div>\n<p class=\"note\">{}</p>\n",
        t("Locus", "Locus"), t("Allel", "Allele"), t("Allel", "Allele"),
        t("Zwei Felder, wie in Klinik und Spenderauswahl üblich; nur von den Reads gestützte Felder. Darunter: G-Gruppe (gleiche Sequenz der Antigen-Bindungsstelle), serologisches Antigen nach WHO (* = angenommen oder durch Experten zugeordnet, nicht serologisch getestet), Ligand der NK-Zell-Rezeptoren und die erreichte Auflösung. Die volle Auflösung steht weiter unten.",
          "Two fields, as used in clinics and donor selection; only fields the reads support. Below: G group (same sequence of the antigen-binding site), serological antigen after WHO (* = assumed or assigned by experts, not tested serologically), ligand of the natural killer cell receptors and the resolution reached. The full resolution follows further down.")
    )
}

/// method and limits in a few lines, with the validation numbers of this version
fn methods(t: &dyn Fn(&'static str, &'static str) -> &'static str) -> String {
    format!(
        "<h2>{}</h2>\n<div class=\"panel prose\"><p>{}</p><p>{}</p><p>{}</p></div>\n",
        t("Methode und Grenzen", "Method and limits"),
        t("Reads aus der Gesamtgenom-Sequenzierung werden gegen alle Allele der IPD-IMGT/HLA-Datenbank verglichen. Für jedes Gen wählt das Programm das Allelpaar, das die Reads am besten erklärt, und gibt je Allel nur die Felder an, bei denen der Vorsprung vor dem nächsten Konkurrenten eine feste, an Assemblies geprüfte Schwelle erreicht. Abweichungen, die die Reads einheitlich zeigen, werden als mögliches neues Allel gemeldet.",
          "Reads from whole-genome sequencing are compared with all alleles of the IPD-IMGT/HLA database. For each gene the program chooses the allele pair that explains the reads best and reports, per allele, only the fields where the lead over the nearest competitor reaches a fixed threshold validated on assemblies. Differences the reads show consistently are reported as a possible new allele."),
        t("Prüfung an 80 Genomen mit vollständig assemblierten Haplotypen (HPRC, 30x): zwei Felder 2486 von 2489 Allelen richtig, gesicherte vier Felder 1740 von 1761 exakt. Prüfung an 59 weiteren, beim Entwickeln nicht verwendeten Genomen (HGSVC3): zwei Felder 1833 von 1833 richtig.",
          "Validation on 80 genomes with fully assembled haplotypes (HPRC, 30x): two fields right for 2486 of 2489 alleles, supported four fields exact for 1740 of 1761. Validation on 59 further genomes not used during development (HGSVC3): two fields right for 1833 of 1833."),
        t("Grenzen kurzer Reads: Welche Allele verschiedener Gene auf demselben Chromosom liegen (Haplotyp), lässt sich nicht bestimmen. Die Länge langer Einzelbasen-Läufe und Wiederholungen ist nicht messbar; Strecken ohne eindeutig zuordenbare Reads bleiben ungeprüft. Ob ein Allel exprimiert wird, folgt nur aus dem Namen in der Datenbank. Neue Allele sollten mit langen Reads oder Sanger-Sequenzierung bestätigt werden.",
          "Limits of short reads: which alleles of different genes lie on the same chromosome (haplotype) cannot be determined. The length of long single-base runs and repeats cannot be measured; stretches without uniquely placed reads stay unchecked. Whether an allele is expressed follows only from its name in the database. New alleles should be confirmed with long reads or Sanger sequencing.")
    )
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
        "<!DOCTYPE html>\n<html lang=\"{first}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>hla-wgs · {}</title>\n<style>{css}</style></head><body data-lang=\"{first}\" data-tab=\"r\">\n<header class=\"bar\"><div class=\"bar-in\"><div class=\"brand\">PJ Labs <i>·</i> hla-wgs</div><nav class=\"lang seg\"><button type=\"button\" data-set=\"de\">Deutsch</button><button type=\"button\" data-set=\"en\">English</button></nav></div></header>\n<main lang=\"de\" class=\"l-de\">\n{}</main>\n<main lang=\"en\" class=\"l-en\">\n{}</main>\n<script>\nfunction aria(){{document.querySelectorAll('nav.lang button').forEach(b=>b.setAttribute('aria-pressed',b.dataset.set===document.body.dataset.lang));document.querySelectorAll('nav.tabs button').forEach(b=>b.setAttribute('aria-pressed',b.dataset.tab===document.body.dataset.tab))}}\nfunction setL(l){{document.body.dataset.lang=l;document.documentElement.lang=l;aria();try{{localStorage.setItem('hla-wgs-lang',l)}}catch(e){{}}}}\ntry{{const s=localStorage.getItem('hla-wgs-lang');if(s==='de'||s==='en')setL(s)}}catch(e){{}}\ndocument.querySelectorAll('nav.lang button').forEach(b=>b.addEventListener('click',()=>setL(b.dataset.set)));\ndocument.querySelectorAll('nav.tabs button').forEach(b=>b.addEventListener('click',()=>{{document.body.dataset.tab=b.dataset.tab;aria();scrollTo(0,0)}}));\ndocument.querySelectorAll('button.copy').forEach(b=>b.addEventListener('click',()=>{{const c=document.getElementById(b.dataset.copy);if(c&&navigator.clipboard)navigator.clipboard.writeText(c.textContent).then(()=>{{const o=b.textContent;b.textContent='✓';setTimeout(()=>b.textContent=o,1200)}})}}));\naria();\naddEventListener('beforeprint',()=>document.querySelectorAll('details').forEach(d=>d.open=true));\n</script></body></html>\n",
        esc(sample),
        render_one(rep, sample, true),
        render_one(rep, sample, false)
    )
}

fn render_one(rep: &Value, sample: &str, de: bool) -> String {
    let t = |d: &'static str, e: &'static str| if de { d } else { e };
    let version = rep["version"].as_str().unwrap_or("");
    let now = rep["analysis_time"].as_u64().unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let (y, m, d) = date(now);
    let day = if de { format!("{d:02}.{m:02}.{y}") } else { format!("{y}-{m:02}-{d:02}") };
    let empty = vec![];
    let genes = rep["genes"].as_array().unwrap_or(&empty);

    let (mut n_alleles, mut n4, mut n_new) = (0, 0, 0);
    let mut details = String::new();
    let thr = |k: &str| rep["thresholds"][k].as_f64().unwrap_or(0.0);
    let f1 = |v: &Value| v.as_f64().map(|x| format!("{x:.1}")).unwrap_or_else(|| "–".to_string());
    let mut rows = String::new();
    let mut rrows = String::new();
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
            let tgt = if c >= 2 { &mut rrows } else { &mut rows };
            tgt.push_str(&format!("<tr class=\"cls\"><td colspan=\"5\">{}</td></tr>\n", names[c as usize]));
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
                let _ = safe;
                cells[i] = format!("<td>{}</td>", name_html(full, res));
                dots.push_str(&format!("<div class=\"d\" title=\"{res} {}\">", t("Felder gesichert", "fields supported")));
                for k in 1..=4 {
                    dots.push_str(if k <= res { "<i class=\"f on\"></i>" } else { "<i class=\"f\"></i>" });
                }
                dots.push_str(&format!(" <span class=\"rw\">{res}/4</span></div>"));
                if let Some(x) = a["expression"].as_str() {
                    notes.push(format!("<span class=\"pos\">{}: {}</span>", esc(full), esc(x)));
                }
                if let Some(n) = new_allele_note(a, i, &either, &t) {
                    gene_new = true;
                    notes.push(n);
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
        (if c >= 2 { &mut rrows } else { &mut rows }).push_str(&format!(
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
            (0, true) => t("negativ", "negative").to_string(),
            (0, false) => format!("<span class=\"warn\">{}</span>", t("nicht sicher beurteilbar", "cannot be judged with certainty")),
            (n, c) => {
                carriers += 1;
                format!("<span class=\"pos\">{} ({}){}</span>", t("positiv", "positive"), if n >= 2 { t("homozygot", "homozygous") } else { t("heterozygot", "heterozygous") }, if c { "" } else { t(", unsicher", ", uncertain") })
            }
        };
        let al = p["allele"].as_str().unwrap_or("");
        let (rec, gl, url) = pgx_rec(al, &t);
        let advice = match (n, certain) {
            (0, true) => t("keine Einschränkung aus dieser Leitlinie", "no restriction from this guideline").to_string(),
            (0, false) => t("Bestätigung durch ein HLA-Labor nötig", "confirmation by an HLA laboratory needed").to_string(),
            _ => format!("<span class=\"pos\">{}</span>", esc(rec)),
        };
        pgx.push_str(&format!(
            "<tr><td class=\"g\">HLA-{}</td><td>{state}</td><td>{}</td><td>{advice}</td><td class=\"note hide\"><a href=\"{url}\">{gl}</a></td></tr>\n",
            esc(al),
            esc(p["drugs"].as_str().unwrap_or(""))
        ));
    }

    // the example for the figure: an allele of this sample with four fields of which not all are supported
    // (shows both kinds), else the first with four fields
    let all: Vec<(String, u64, Value)> = genes.iter().filter(|g| g["copies"].as_u64().unwrap_or(2) > 0).flat_map(|g| g["alleles"].as_array().cloned().unwrap_or_default()).map(|a| (a["allele"].as_str().unwrap_or("").to_string(), a["resolution"].as_u64().unwrap_or(0), a)).filter(|(n, _, _)| n.split(':').count() == 4).collect();
    let example = all.iter().find(|(_, r, _)| *r == 3).or(all.first()).cloned().unwrap_or_else(|| ("A*02:01:01:01".to_string(), 4, Value::Null));
    let mut h = String::with_capacity(32_000);
    h.push_str(&format!(
        "{}",
        ""
    ));
    h.push_str(&format!("<div class=\"ruo\">{}</div>\n", t(
        "Nur für Forschung (RUO). Kein medizinischer Befund und kein Medizinprodukt.",
        "Research use only (RUO). Not a medical report and not a medical device."
    )));
    h.push_str(&format!(
        "<div class=\"eyebrow\">{}</div><h1>{}</h1>\n",
        t("HLA-Typisierung aus dem Genom", "HLA typing from the genome"),
        esc(sample)
    ));
    // report head as in a laboratory: what was analysed, with what, and how well
    let depth = rep["depth_estimate"].as_f64();
    let mask = rep["repeat_mask"].as_bool().unwrap_or(true);
    let clin_ok = CLINICAL.iter().filter(|c| !matches!(**c, "DRB3" | "DRB4" | "DRB5")).all(|c| genes.iter().any(|g| g["gene"] == *c && g["called"].as_bool().unwrap_or(false)));
    let mut limits: Vec<&str> = vec![];
    if depth.map_or(false, |d| d < 18.0) { limits.push(t("geringe Tiefe", "low depth")); }
    if !mask { limits.push(t("ohne Wiederholungsmaske", "without repeat mask")); }
    if !rep["groups_loaded"].as_bool().unwrap_or(true) { limits.push(t("ohne G/P-Gruppen", "without G/P groups")); }
    if !clin_ok { limits.push(t("klinischer Locus ohne Ergebnis", "clinical locus without result")); }
    let qc = if limits.is_empty() { format!("<span class=\"ok\">{}</span>", t("bestanden", "passed")) } else { format!("<span class=\"warn\">{}: {}</span>", t("eingeschränkt", "limited"), limits.join(", ")) };
    let input = rep["input"].as_array().map(|v| v.iter().filter_map(|x| x.as_str()).map(esc).collect::<Vec<_>>().join(", ")).filter(|x| !x.is_empty()).unwrap_or_else(|| "–".into());
    let kv = [
        (t("Probe", "Sample"), esc(sample)),
        (t("Berichtsnummer", "Report number"), report_id(rep)),
        (t("Eingabe", "Input"), input),
        (t("Analyse", "Analysis"), day.clone()),
        (t("Programm", "Software"), format!("hla-wgs {}", esc(version))),
        (t("Datenbank", "Database"), rep["database_release"].as_str().map(|r| format!("IPD-IMGT/HLA {}", esc(r))).unwrap_or_else(|| "IPD-IMGT/HLA".into())),
        (t("Tiefe über HLA", "Depth over HLA"), depth.map(|d| format!("≈ {d:.0}x")).unwrap_or_else(|| "–".into())),
        (t("Qualität", "Quality"), qc),
    ];
    h.push_str("<div class=\"head\">");
    for (k, v) in kv { h.push_str(&format!("<div><span>{k}</span><b>{v}</b></div>")); }
    h.push_str("</div>\n");
    if rep["repeat_mask"].as_bool() == Some(false) {
        h.push_str(&format!("<div class=\"alert\">{}</div>\n", t(
            "Ohne Wiederholungsmaske gerechnet: Reads aus ähnlichen Stellen des Genoms können mitgezählt werden. Das Ergebnis ist weniger zuverlässig als in der Prüfung (zu viele Felder bei MICA/MICB, scheinbare Abweichungen bei DRB1/DRB4). Maske einrichten (hla-wgs build-mask) und neu starten.",
            "Run without the repeat mask: reads from similar places in the genome can be counted. The result is less reliable than in the validation (too many fields for MICA/MICB, apparent differences in DRB1/DRB4). Set up the mask (hla-wgs build-mask) and run again."
        )));
    }
    if let Some(dx) = rep["depth_estimate"].as_f64().filter(|d| *d < 18.0) {
        // validation, 80 HPRC samples thinned (validation/results/depth): certain four-field calls exact
        // 98.8 % at 30x and 20x, 98.6 % at 15x, 97.9 % at 10x, and about 40 % fewer of them at 10x
        let msg = if de {
            format!("Geringe Tiefe: etwa {dx:.0}x über den HLA-Genen. Das Programm sichert dann weniger Felder, und gesicherte vierte Felder sind etwas seltener exakt (Prüfung bei 10x: 97,9 % statt 98,8 % bei 30x). Für ein verlässliches viertes Feld sind 15x oder mehr ratsam.")
        } else {
            format!("Low depth: about {dx:.0}x over the HLA genes. Fewer fields are then supported, and supported fourth fields are somewhat less often exact (validation at 10x: 97.9 % instead of 98.8 % at 30x). For a dependable fourth field, 15x or more is advisable.")
        };
        h.push_str(&format!("<div class=\"alert soft\">{}</div>\n", esc(&msg)));
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
    h.push_str(&format!("<h2>{}</h2>\n{}", t("Klinische Kurzfassung", "Clinical summary"), clinical(genes, &t)));
    let gl = gl_string(genes);
    if !gl.is_empty() {
        h.push_str(&format!(
            "<h2>{}</h2>\n<div class=\"panel gl\"><code id=\"gl-{}\">{}</code><button type=\"button\" class=\"copy\" data-copy=\"gl-{}\">{}</button></div>\n<p class=\"note\">{}</p>\n",
            t("GL-String", "GL String"), if de { "de" } else { "en" }, esc(&gl), if de { "de" } else { "en" }, t("Kopieren", "Copy"),
            t("Genotype List String, das Austauschformat für HLA-Typisierungen (Loci mit ^, beide Kopien mit + verbunden); nur gestützte Felder.", "Genotype List String, the exchange format for HLA typings (loci joined by ^, both copies by +); supported fields only.")
        ));
    }
    if !pgx.is_empty() {
        h.push_str(&format!(
            "<h2>{}</h2>\n<div class=\"panel\"><table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th class=\"hide\">{}</th></tr></thead><tbody>\n{pgx}</tbody></table></div>\n<p class=\"note\">{}</p>\n",
            t("Pharmakogenetik", "Pharmacogenetics"),
            t("Allel", "Allele"),
            t("Befund", "Finding"),
            t("Wirkstoffe", "Drugs"),
            t("Empfehlung der Leitlinie", "Guideline recommendation"),
            t("Leitlinie", "Guideline"),
            t("Empfehlungen des Clinical Pharmacogenetics Implementation Consortium (CPIC), gekürzt; sie gelten für den Beginn einer Behandlung. Eine Arzneimittel-Entscheidung braucht ärztliche Beurteilung und einen bestätigten Befund.",
              "Recommendations of the Clinical Pharmacogenetics Implementation Consortium (CPIC), shortened; they apply when starting a treatment. A drug decision needs medical judgement and a confirmed result.")
        ));
    }
    h.push_str(&format!(
        "<h2>{}</h2>\n{}<div class=\"panel\"><table class=\"tf\"><thead><tr><th>{}</th><th>{} 1</th><th>{} 2</th><th>{}</th><th class=\"hide\">{}</th></tr></thead><tbody>\n{rows}</tbody></table></div>\n<p class=\"note\">{}</p>\n",
        t("Volle Auflösung", "Full resolution"),
        legend(&t, &example, &thr),
        t("Gen", "Gene"),
        t("Allel", "Allele"),
        t("Allel", "Allele"),
        t("Felder", "Fields"),
        t("Hinweis", "Note"),
        t(
            "Farbig sind die Felder, die die Reads sichern (der sichere Ruf); grau sind Felder des nächstliegenden Datenbank-Allels, die nicht gesichert sind. Die Punkte und die Zahl daneben zählen die gesicherten Felder.",
            "Coloured are the fields the reads support (the safe call); grey are fields of the nearest database allele that are not supported. The dots and the number beside them count the supported fields."
        )
    ));
    if !rrows.is_empty() {
        h.push_str(&format!(
            "<details class=\"more\"><summary><b>{}</b> <span class=\"note\">{}</span></summary><table class=\"tf\"><tbody>\n{rrows}</tbody></table></details>\n",
            t("Weitere Gene", "Further genes"),
            t("nicht klassische HLA-Gene, Pseudogene, MICA/MICB, TAP, HFE: für die Forschung", "non-classical HLA genes, pseudogenes, MICA/MICB, TAP, HFE: for research")
        ));
    }
    h.push_str(&methods(&t));
    h.push_str(&format!(
        "</section>\n<section class=\"tab-d\">\n<h2>{}</h2>\n<p class=\"note\">{}</p>\n{details}</section>\n",
        t("Details je Gen", "Details per gene"),
        t(
            "Allel 1 und Allel 2 sind die beiden Kopien des Gens, eine von der Mutter und eine vom Vater; die Nummer sagt nicht, von wem welche stammt. Alle Werte aus .tsv und .conf.tsv. Vorsprung: wie viel besser (Log-Likelihood) das gewählte Allel die Reads erklärt als das beste Allel, das sich in diesem Feld unterscheidet; ab der Schwelle gilt das Feld als gesichert. Ein Klick öffnet ein Gen; beim Drucken sind alle offen.",
            "Allele 1 and allele 2 are the two copies of the gene, one from the mother and one from the father; the number does not say which is which. All values of .tsv and .conf.tsv. Lead: how much better (log-likelihood) the chosen allele explains the reads than the best allele that differs in this field; at or above the threshold the field counts as supported. Click a gene to open it; all are open when printed."
        )
    ));
    let rel = rep["database_release"].as_str().map(|r| format!(" {} {}.", t("Datenbank: IPD-IMGT/HLA", "Database: IPD-IMGT/HLA"), esc(r))).unwrap_or_default();
    h.push_str(&format!(
        "<footer>\n<p class=\"cite\">{}: Jungwirth P. hla-wgs: HLA typing from whole-genome sequencing with a per-allele statement of what is certain. Zenodo. <a href=\"https://doi.org/{DOI}\">https://doi.org/{DOI}</a></p>\n<p>{}: <a href=\"{ARTICLE}\">{ARTICLE}</a> · {}: <a href=\"{SOURCE}\">{SOURCE}</a></p>\n<p>{}{rel}</p>\n<p>{}</p>\n</footer>\n",
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
    let calls: Vec<String> = als.iter().take(copies.min(2) as usize).map(|a| name_html(a["allele"].as_str().unwrap_or(""), a["resolution"].as_u64().unwrap_or(0))).collect();
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
        let res = a["resolution"].as_u64().unwrap_or(0);
        let labels: Vec<String> = (1..=4usize).map(|k| {
            let m = if k == 1 { None } else { a["margin"][k.to_string().as_str()].as_f64() };
            let st = if (k as u64) <= res { t("✓ gesichert", "✓ supported") } else { t("? nicht gesichert", "? not supported") };
            if (k as u64) == res + 1 {
                // the first missing field points to the reason, which is not always the lead
                return format!("{} · {st}: {}", field_meaning(t, k), short_reason(a, t, thr));
            }
            match m {
                Some(m) if (k as u64) <= res => format!("{} · {st} ({} {m:.1}, {} {:.0})", field_meaning(t, k), t("Vorsprung", "lead"), t("Schwelle", "threshold"), thr(&k.to_string())),
                _ => format!("{} · {st}", field_meaning(t, k)),
            }
        }).collect();
        o.push_str(&format!("<h4 class=\"ah\">{}</h4>\n", if copies >= 2 { format!("{} {}", t("Allel", "Allele"), i + 1) } else { t("Allel (eine Kopie)", "Allele (one copy)").to_string() }));
        o.push_str(&field_diagram(full, res, &labels));
        o.push_str(&format!(
            "<div class=\"kv\">{}: {} · {}: {} · G: {} · P: {}{}{}</div>\n",
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
                let mut nt: Vec<String> = vec![];
                let seen = !ch.starts_with("blind");
                let reg = x["region"].as_str().filter(|r| *r != "-" && seen).map(|r| r.replace("exon", t("Exon", "exon")).replace("intron", t("Intron", "intron")));
                match reg {
                    Some(r) => nt.push(r),
                    None => if x["exon"].as_bool().unwrap_or(false) { nt.push(t("im Exon", "in an exon").to_string()) },
                }
                if let Some(e) = x["effect"].as_str().filter(|e| *e != "-" && seen) { nt.push(effect_text(e, t)); }
                if x["either_allele"].as_bool().unwrap_or(false) { nt.push(t("keiner Kopie sicher zuzuordnen", "cannot be assigned to one copy").to_string()); }
                if ch.starts_with("blind") { nt.push(t("Strecke ohne eindeutig zuordenbare Reads", "stretch without uniquely placed reads").to_string()); }
                if ch.contains("unsupported") { nt.push(t("Datenbank-Base von keinem Read gestützt", "database base backed by no read").to_string()); }
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
/// effect of a new site on the protein, in the page language (the JSON carries the English text)
fn effect_text(e: &str, t: &dyn Fn(&'static str, &'static str) -> &'static str) -> String {
    for (en, de) in [
        ("non-coding", "nicht kodierend"), ("splice site", "Spleißstelle"), ("synonymous", "still"), ("missense", "Aminosäure-Austausch"),
        ("stop gained", "Stopp entsteht"), ("stop lost", "Stopp entfällt"), ("start codon lost", "Startcodon entfällt"),
        ("frameshift", "Leseraster verschoben"), ("in-frame length change", "Längenänderung im Leseraster"), ("after an early stop codon", "hinter einem vorzeitigen Stoppcodon"),
    ] {
        if let Some(rest) = e.strip_prefix(en) {
            return format!("{}{}", t(de, en), esc(rest));
        }
    }
    esc(e)
}

fn field_reason(_gene: &str, a: &Value, t: &dyn Fn(&'static str, &'static str) -> &'static str, thr: &dyn Fn(&str) -> f64) -> String {
    let r = a["resolution"].as_u64().unwrap_or(0);
    let nf = a["allele"].as_str().and_then(|x| x.split('*').nth(1)).map(|x| x.split(':').count() as u64).unwrap_or(4);
    if r >= 4 || r >= nf {
        return t("alle Felder des Allelnamens gesichert", "all fields of the allele name are supported").to_string();
    }
    if r == 0 {
        return format!("{} {}", t("auch die Allelgruppe ist nicht gesichert: ein Allel einer anderen Gruppe erklärt die Reads fast gleich gut", "the allele group is not supported either: an allele of another group explains the reads almost as well"), esc(a["alternative"]["2"].as_str().unwrap_or("")));
    }
    let next = (r + 1).max(2).to_string();
    if let Some(m) = a["margin"][next.as_str()].as_f64() {
        if m < thr(&next) {
            return format!("{} {next}: {m:.1} < {:.0}", t("Vorsprung vor dem nächsten Konkurrenten unter der Schwelle im Feld", "lead over the nearest competitor below the threshold at field"), thr(&next));
        }
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
    #[test]
    fn clinical_summary_and_gl_string() {
        let g = |n: &str, a: &str, b: &str| serde_json::json!({"gene": n, "called": true, "copies": 2, "alleles": [
            {"allele": a, "safe_call": a, "resolution": 4, "serology": "B60 (B40)", "ligand": "Bw6", "new_sites": []},
            {"allele": b, "safe_call": b.rsplitn(2, ':').nth(1).unwrap(), "resolution": 3, "new_sites": []}]});
        let genes = vec![g("A", "A*02:01:01:01", "A*01:01:01:01"), g("B", "B*40:01:02:01", "B*08:01:01:01"),
            serde_json::json!({"gene": "DRB4", "called": true, "copies": 0, "alleles": []})];
        assert_eq!(super::gl_string(&genes), "HLA-A*02:01:01:01+HLA-A*01:01:01^HLA-B*40:01:02:01+HLA-B*08:01:01");
        assert_eq!(super::two_field("B*40:01:02"), "HLA-B*40:01");
        let t = |d: &'static str, _e: &'static str| d;
        let c = super::clinical(&genes, &t);
        assert!(c.contains("HLA-B*40:01") && c.contains("B60 (B40)") && c.contains("Bw6") && c.contains("keines vorhanden"));
        assert!(!c.contains("HLA-DRB4<"));
    }
}
