const LINKEDIN_URL: &str = "https://www.linkedin.com/in/bj%C3%B6rn-molin-1843aa3";
const GITHUB_URL: &str = "https://github.com/motrice/sopor";
const AUTHOR: &str = "Björn Molin";

const HEAD_OPEN: &str = r#"<!doctype html>
<html lang="sv">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
"#;

const HEAD_STYLE: &str = r#"<style>
  :root { --fg:#1a1a1a; --muted:#666; --bg:#fafaf7; --card:#fff; --acc:#2f6f3d; --border:#e3e3dc; }
  * { box-sizing: border-box; }
  body { margin:0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
         background: var(--bg); color: var(--fg); line-height: 1.5; }
  main { max-width: 640px; margin: 3rem auto; padding: 0 1rem; }
  h1 { font-size: 1.6rem; margin: 0 0 .25rem 0; }
  p.lede { color: var(--muted); margin: 0 0 2rem 0; }
  .card { background: var(--card); border: 1px solid var(--border); border-radius: 12px;
          padding: 1.25rem; box-shadow: 0 1px 2px rgba(0,0,0,.03); }
  label { display:block; font-weight: 600; margin-bottom: .35rem; }
  .search { position: relative; }
  input[type=text] { width:100%; padding: .65rem .8rem; font-size: 1rem;
                     border: 1px solid var(--border); border-radius: 8px; background: #fff; }
  input[type=text]:focus { outline: 2px solid var(--acc); outline-offset: -1px; }
  .suggestions { position:absolute; left:0; right:0; top: calc(100% + 4px);
                 background:#fff; border:1px solid var(--border); border-radius: 8px;
                 max-height: 280px; overflow:auto; z-index: 10;
                 box-shadow: 0 8px 20px rgba(0,0,0,.06); }
  .suggestions div { padding: .55rem .8rem; cursor: pointer; }
  .suggestions div:hover, .suggestions div.active { background: #f0f4ee; }
  .hidden { display:none; }
  .result { margin-top: 1.5rem; }
  .pickups { display:grid; gap:.5rem; margin: 1rem 0; }
  .pickup { display:flex; justify-content:space-between; padding:.6rem .8rem;
            background:#f6f7f3; border-radius: 8px; font-size: .95rem; }
  .pickup b { color: var(--acc); }
  .url-box { display:flex; gap:.5rem; margin-top: 1rem; }
  .url-box input { flex:1; font-family: ui-monospace, SFMono-Regular, monospace; font-size: .85rem; }
  button { padding: .6rem 1rem; border-radius: 8px; border:0; cursor:pointer;
           background: var(--acc); color:#fff; font-weight:600; font-size: .9rem; }
  button.ghost { background:#fff; color: var(--fg); border:1px solid var(--border); }
  .actions { display:flex; flex-wrap:wrap; gap:.5rem; margin-top: 1rem; }
  .actions a { text-decoration: none; }
  .empty { color: var(--muted); font-style: italic; }
  footer { margin-top: 2rem; color: var(--muted); font-size: .85rem; text-align:center; }
  footer a { color: inherit; }
  .note { font-size:.8rem; color: var(--muted); margin-top: .5rem; }
  .kommun-list { list-style: none; padding: 0; margin: 0; display: grid; gap: .5rem; }
  .kommun-list a { display: block; padding: .8rem 1rem; background: #f6f7f3;
                   border-radius: 8px; text-decoration: none; color: var(--fg);
                   font-weight: 600; border: 1px solid var(--border); }
  .kommun-list a:hover { background: #f0f4ee; border-color: var(--acc); }
  .back { color: var(--muted); text-decoration: none; font-size: .85rem; }
  .back:hover { text-decoration: underline; }
  .pitch-link { margin: 0 0 1.25rem 0; }
  .pitch-link a { display: inline-block; padding: .5rem .85rem; background: #eef4ea;
                  border: 1px solid var(--border); border-left: 3px solid var(--acc);
                  border-radius: 6px; color: var(--acc); text-decoration: none;
                  font-weight: 600; font-size: .9rem; }
  .pitch-link a:hover { background: #e3ecdd; }
  .byline { margin: 0 0 .75rem 0; color: var(--muted); font-size: .85rem; }
  .byline a { color: var(--acc); text-decoration: none; font-weight: 600; }
  .byline a:hover { text-decoration: underline; }
</style>
</head>
<body>
"#;

const TAIL: &str = r#"
</body>
</html>
"#;

fn head(title: &str, extra_meta: &str) -> String {
    let mut s = String::new();
    s.push_str(HEAD_OPEN);
    s.push_str("<title>");
    s.push_str(&escape(title));
    s.push_str("</title>\n");
    s.push_str(extra_meta);
    s.push_str(HEAD_STYLE);
    s
}

pub fn render_index(base_url: &str, kommuner: &[(&str, &str)]) -> String {
    let title = "Sophämtning i din kalender — Google, Apple, Outlook";
    let desc = format!(
        "Prenumerera på sophämtningsschemat för din adress i {} svenska kommuner. \
         Notis kvällen innan. Fungerar med Google Calendar, Apple Kalender och \
         Outlook. Ingen inloggning, ingen app.",
        kommuner.len()
    );
    let img_url = format!("{base_url}/index-og.jpg");
    let og = format!(
        r##"<meta name="description" content="{desc}">
<link rel="canonical" href="{url}">

<meta property="og:type" content="website">
<meta property="og:site_name" content="sopkalender.se">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{url}">
<meta property="og:image" content="{img}">
<meta property="og:image:type" content="image/jpeg">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="627">
<meta property="og:image:alt" content="Fyra färgade sopkärl (matavfall, restavfall, pappersförpackningar, plastförpackningar) med olika hämtningsdagar och kalendericoner — sophämtning som kalenderprenumeration.">
<meta property="og:locale" content="sv_SE">

<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{title}">
<meta name="twitter:description" content="{desc}">
<meta name="twitter:image" content="{img}">

"##,
        title = escape(title),
        desc = escape(&desc),
        url = escape(base_url),
        img = escape(&img_url),
    );
    let mut out = String::new();
    out.push_str(&head(title, &og));
    out.push_str(&format!(
        r#"<main>
  <p class="byline">Av <a href="{linkedin}" target="_blank" rel="noopener">{author}</a> ·
     <a href="{linkedin}" target="_blank" rel="noopener">LinkedIn</a> ·
     <a href="{github}" target="_blank" rel="noopener">GitHub</a></p>
  <p class="pitch-link"><a href="/pitch">Varför är detta så svårt? Om öppna sopdata i Sverige →</a></p>
  <h1>Sophämtningskalender</h1>
  <p class="lede">Välj din kommun för att skapa en kalenderprenumeration
    med dina sophämtningsdatum.</p>

  <div class="card">
    <label>Kommuner</label>
    <ul class="kommun-list">
"#,
        linkedin = LINKEDIN_URL,
        github = GITHUB_URL,
        author = AUTHOR,
    ));
    for (id, name) in kommuner {
        out.push_str(&format!(
            r#"      <li><a href="/{id}">{name}</a></li>
"#,
            id = escape(id),
            name = escape(name)
        ));
    }
    out.push_str(
        r#"    </ul>
  </div>

  <footer>
    Inofficiell tjänst — kontakta din kommun för officiella uppgifter.<br>
    100% vibe kodat
  </footer>
</main>"#,
    );
    out.push_str(TAIL);
    out
}

pub fn render_kommun(slug: &str, name: &str, placeholder: &str, note: &str) -> String {
    let mut out = String::new();
    out.push_str(&head("Sopor — kalenderprenumeration för sophämtning", ""));
    out.push_str(&format!(
        r#"<main>
  <a href="/" class="back">← Alla kommuner</a>
  <h1>Sophämtningskalender — {name}</h1>
  <p class="lede">{note}</p>

  <div class="card">
    <label for="q">Adress</label>
    <div class="search">
      <input id="q" type="text" autocomplete="off" placeholder="{placeholder}">
      <div id="suggestions" class="suggestions hidden"></div>
    </div>
    <div id="result" class="result"></div>
  </div>

  <footer>
    Inofficiell tjänst — kontakta {name}s renhållning för officiella uppgifter.
  </footer>
</main>

<script>
const SLUG = {slug_json};
const q = document.getElementById('q');
const sugg = document.getElementById('suggestions');
const result = document.getElementById('result');
let timer = null;
let active = -1;
let suggestions = [];

q.addEventListener('input', () => {{
  clearTimeout(timer);
  const v = q.value.trim();
  if (v.length < 2) {{ hideSuggestions(); return; }}
  timer = setTimeout(() => fetchSuggestions(v), 250);
}});

q.addEventListener('keydown', (e) => {{
  if (sugg.classList.contains('hidden')) return;
  if (e.key === 'ArrowDown') {{ e.preventDefault(); active = Math.min(active + 1, suggestions.length - 1); renderActive(); }}
  else if (e.key === 'ArrowUp') {{ e.preventDefault(); active = Math.max(active - 1, 0); renderActive(); }}
  else if (e.key === 'Enter') {{
    e.preventDefault();
    if (active >= 0 && suggestions[active]) selectAddress(suggestions[active].value);
    else if (suggestions[0]) selectAddress(suggestions[0].value);
  }} else if (e.key === 'Escape') hideSuggestions();
}});

document.addEventListener('click', (e) => {{
  if (!sugg.contains(e.target) && e.target !== q) hideSuggestions();
}});

async function fetchSuggestions(v) {{
  try {{
    const res = await fetch('/' + SLUG + '/autocomplete?query=' + encodeURIComponent(v));
    if (!res.ok) return;
    suggestions = await res.json();
    if (!suggestions.length) {{ hideSuggestions(); return; }}
    sugg.innerHTML = '';
    suggestions.forEach((s) => {{
      const div = document.createElement('div');
      div.textContent = s.value;
      div.addEventListener('mousedown', (ev) => {{ ev.preventDefault(); selectAddress(s.value); }});
      sugg.appendChild(div);
    }});
    active = -1;
    sugg.classList.remove('hidden');
  }} catch (e) {{ console.error(e); }}
}}

function renderActive() {{
  [...sugg.children].forEach((el, i) => el.classList.toggle('active', i === active));
}}

function hideSuggestions() {{ sugg.classList.add('hidden'); active = -1; }}

async function selectAddress(address) {{
  q.value = address;
  hideSuggestions();
  result.innerHTML = '<p class="empty">Söker hämtningsinformation…</p>';
  try {{
    const res = await fetch('/' + SLUG + '/preview?address=' + encodeURIComponent(address));
    if (!res.ok) {{ result.innerHTML = '<p class="empty">Något gick fel.</p>'; return; }}
    const data = await res.json();
    renderResult(address, data);
  }} catch (e) {{
    result.innerHTML = '<p class="empty">Något gick fel.</p>';
  }}
}}

function renderResult(address, data) {{
  if (!data.length) {{
    result.innerHTML = '<p class="empty">Inga hämtningar hittades för adressen.</p>';
    return;
  }}
  const url = location.origin + '/' + SLUG + '/ics?address=' + encodeURIComponent(address);
  const webcal = 'webcal://' + location.host + '/' + SLUG + '/ics?address=' + encodeURIComponent(address);
  const gcalUrl = 'https://calendar.google.com/calendar/u/0/r?cid=' + encodeURIComponent(webcal);
  const outlookUrl = 'https://outlook.live.com/calendar/0/addfromweb?url=' +
                     encodeURIComponent(url) + '&name=' + encodeURIComponent('Sophämtning ' + address);

  let html = '<h2 style="font-size:1.1rem;margin:0 0 .5rem 0">' + escapeHtml(address) + '</h2>';
  html += '<div class="pickups">';
  for (const series of data) {{
    if (!series.entries.length) continue;
    const next = series.entries[0];
    const freq = series.frequency ? ' — ' + escapeHtml(series.frequency) : '';
    html += '<div class="pickup"><span>' + escapeHtml(series.waste_type) + freq +
            '</span><b>' + escapeHtml(next.date) + ' (' + escapeHtml(next.weekday) + ')</b></div>';
  }}
  html += '</div>';
  html += '<label>Prenumerationslänk</label>';
  html += '<div class="url-box"><input type="text" id="ics-url" readonly value="' + escapeAttr(url) + '">';
  html += '<button onclick="copyUrl()">Kopiera</button></div>';
  html += '<p class="note">Lägg till URL:en i din kalenderapp som en prenumeration. Den uppdateras automatiskt.</p>';
  html += '<div class="actions">';
  html += '<a href="' + escapeAttr(gcalUrl) + '" target="_blank"><button>Lägg till i Google Calendar</button></a>';
  html += '<a href="' + escapeAttr(outlookUrl) + '" target="_blank"><button class="ghost">Lägg till i Outlook</button></a>';
  html += '<a href="' + escapeAttr(webcal) + '"><button class="ghost">Apple Calendar (webcal)</button></a>';
  html += '<a href="' + escapeAttr(url) + '" download="sophamtning.ics"><button class="ghost">Ladda ner .ics</button></a>';
  html += '</div>';
  result.innerHTML = html;
}}

function copyUrl() {{
  const el = document.getElementById('ics-url');
  el.select(); document.execCommand('copy');
}}

function escapeHtml(s) {{
  return String(s).replace(/[&<>"']/g, c => ({{ '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;' }}[c]));
}}
function escapeAttr(s) {{ return escapeHtml(s); }}
</script>"#,
        name = escape(name),
        note = autolink(&escape(note)),
        placeholder = escape(placeholder),
        slug_json = serde_json::to_string(slug).unwrap_or_else(|_| "\"\"".into()),
    ));
    out.push_str(TAIL);
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Wrap bare `https?://…` URLs as clickable links. Input is expected to
/// already be HTML-escaped; the regex stops at whitespace and common
/// trailing punctuation so surrounding sentence punctuation stays outside
/// the anchor.
fn autolink(escaped: &str) -> String {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"https?://[^\s<>]+").unwrap());
    re.replace_all(escaped, |c: &regex::Captures| {
        let raw = &c[0];
        let (url, trail) = match raw.rfind(|ch: char| !matches!(ch, '.' | ',' | ')' | ';' | ':' | '!' | '?')) {
            Some(i) => (&raw[..=i], &raw[i + 1..]),
            None => (raw, ""),
        };
        format!("<a href=\"{url}\" target=\"_blank\" rel=\"noopener\">{url}</a>{trail}")
    })
    .into_owned()
}

const PITCH_BODY: &str = include_str!("../assets/pitch.html");

pub fn render_pitch(base_url: &str) -> String {
    let title = "Varför har bara 1 av 290 svenska kommuner öppen sopdata?";
    let desc = "Jag ville bara ha en kalenderprenumeration på när soporna hämtas. \
                Det blev en resa genom BankID-portaler, PHP 5.4-scrapers och \
                brandväggar som blockerar Google Cloud. Sundsvall visar att det \
                tar en eftermiddag — om man vill.";
    let page_url = format!("{base_url}/pitch");
    let img_url = format!("{base_url}/pitch-og.jpg");
    format!(
        r##"<!doctype html>
<html lang="sv">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{desc}">
<link rel="canonical" href="{page_url}">

<meta property="og:type" content="article">
<meta property="og:site_name" content="sopkalender.se">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{page_url}">
<meta property="og:image" content="{img_url}">
<meta property="og:image:type" content="image/jpeg">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="627">
<meta property="og:image:alt" content="Infografik: Sundsvall som lysande öppna-data-stjärna med höga poäng (98/100, 96/100, 95/100), bredvid tre ledsna soptunnor som representerar andra kommuner med lägre tillgänglighet.">
<meta property="og:locale" content="sv_SE">

<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="{title}">
<meta name="twitter:description" content="{desc}">
<meta name="twitter:image" content="{img_url}">

<style>
  :root {{ --fg:#1a1a1a; --muted:#666; --bg:#fafaf7; --card:#fff;
          --acc:#2f6f3d; --border:#e3e3dc; --star:#c07a00; }}
  * {{ box-sizing: border-box; }}
  body {{ margin:0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
         background: var(--bg); color: var(--fg); line-height: 1.65; }}
  article {{ max-width: 760px; margin: 3rem auto 4rem; padding: 0 1.25rem; }}
  h1 {{ font-size: 2.2rem; line-height: 1.2; margin: 0 0 2rem 0; }}
  h2 {{ font-size: 1.5rem; margin: 2.5rem 0 1rem 0; border-top: 1px solid var(--border); padding-top: 1.75rem; }}
  h3 {{ font-size: 1.15rem; margin: 1.75rem 0 .75rem 0; }}
  p {{ margin: 0 0 1rem 0; }}
  ul, ol {{ padding-left: 1.4rem; margin: 0 0 1rem 0; }}
  li {{ margin-bottom: .5rem; }}
  a {{ color: var(--acc); }}
  code {{ font-family: ui-monospace, SFMono-Regular, monospace; font-size: .9em;
         background: #eef1ec; padding: 1px 5px; border-radius: 3px; }}
  strong {{ color: #111; }}
  .table-wrap {{ overflow-x: auto; margin: 1rem -.25rem 1.5rem; }}
  table {{ width: 100%; border-collapse: collapse; font-size: .92rem; background: var(--card);
          border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }}
  th, td {{ text-align: left; padding: .7rem .85rem; border-bottom: 1px solid var(--border);
           vertical-align: top; }}
  th {{ background: #f0f4ee; font-weight: 700; }}
  tr:last-child td {{ border-bottom: 0; }}
  .closer {{ font-size: 1.15rem; margin-top: 2rem; }}
  .cta {{ margin-top: 2rem; }}
  .cta a {{ display: inline-block; background: var(--acc); color: #fff; padding: .75rem 1.25rem;
           border-radius: 8px; text-decoration: none; font-weight: 700; }}
  .pmpc {{ margin-top: 2rem; padding-top: 1.5rem; border-top: 1px solid var(--border);
           font-size: .9rem; color: var(--muted); text-align: center; }}
  .pmpc img {{ max-width: 100%; height: auto; margin-bottom: .75rem; }}
  .pmpc-credit {{ font-size: .8rem; }}
  footer {{ max-width: 760px; margin: 0 auto 3rem; padding: 0 1.25rem; color: var(--muted);
           font-size: .85rem; text-align: center; }}
  .author {{ max-width: 760px; margin: 0 auto 2rem; padding: 1.5rem 1.25rem 0;
             border-top: 1px solid var(--border); color: var(--muted); font-size: .95rem; }}
  .author a {{ color: var(--acc); font-weight: 600; text-decoration: none; }}
  .author a:hover {{ text-decoration: underline; }}
</style>
</head>
<body>
{body}
<p class="author">Skriven av <a href="{linkedin}" target="_blank" rel="noopener">{author}</a> —
   <a href="{linkedin}" target="_blank" rel="noopener">LinkedIn</a> ·
   <a href="{github}" target="_blank" rel="noopener">Källkod på GitHub</a></p>
<footer>
  <a href="/">← Tillbaka till sopkalender.se</a><br>
  100% vibe kodat
</footer>
</body>
</html>
"##,
        title = escape(title),
        desc = escape(desc),
        page_url = escape(&page_url),
        img_url = escape(&img_url),
        body = PITCH_BODY,
        linkedin = LINKEDIN_URL,
        github = GITHUB_URL,
        author = AUTHOR,
    )
}
