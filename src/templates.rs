pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="sv">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sopor — kalenderprenumeration för sophämtning</title>
<style>
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
</style>
</head>
<body>
<main>
  <h1>Sophämtningskalender</h1>
  <p class="lede">Sök upp en adress och prenumerera på sophämtningsdatum
    i Google Calendar, Apple Calendar eller Outlook. Data från Stockholm Vatten och Avfall.</p>

  <div class="card">
    <label for="q">Adress</label>
    <div class="search">
      <input id="q" type="text" autocomplete="off" placeholder="t.ex. Olovslundsvägen 9">
      <div id="suggestions" class="suggestions hidden"></div>
    </div>

    <div id="result" class="result"></div>
  </div>

  <footer>
    Endast Stockholms stad (villor och radhus). Inofficiell tjänst.
  </footer>
</main>

<script>
const q = document.getElementById('q');
const sugg = document.getElementById('suggestions');
const result = document.getElementById('result');
let timer = null;
let active = -1;
let suggestions = [];

q.addEventListener('input', () => {
  clearTimeout(timer);
  const v = q.value.trim();
  if (v.length < 2) { hideSuggestions(); return; }
  timer = setTimeout(() => fetchSuggestions(v), 250);
});

q.addEventListener('keydown', (e) => {
  if (sugg.classList.contains('hidden')) return;
  if (e.key === 'ArrowDown') { e.preventDefault(); active = Math.min(active + 1, suggestions.length - 1); renderActive(); }
  else if (e.key === 'ArrowUp') { e.preventDefault(); active = Math.max(active - 1, 0); renderActive(); }
  else if (e.key === 'Enter') {
    e.preventDefault();
    if (active >= 0 && suggestions[active]) selectAddress(suggestions[active].value);
    else if (suggestions[0]) selectAddress(suggestions[0].value);
  } else if (e.key === 'Escape') hideSuggestions();
});

document.addEventListener('click', (e) => {
  if (!sugg.contains(e.target) && e.target !== q) hideSuggestions();
});

async function fetchSuggestions(v) {
  try {
    const res = await fetch('/autocomplete?query=' + encodeURIComponent(v));
    if (!res.ok) return;
    suggestions = await res.json();
    if (!suggestions.length) { hideSuggestions(); return; }
    sugg.innerHTML = '';
    suggestions.forEach((s, i) => {
      const div = document.createElement('div');
      div.textContent = s.value;
      div.addEventListener('mousedown', (ev) => { ev.preventDefault(); selectAddress(s.value); });
      sugg.appendChild(div);
    });
    active = -1;
    sugg.classList.remove('hidden');
  } catch (e) { console.error(e); }
}

function renderActive() {
  [...sugg.children].forEach((el, i) => el.classList.toggle('active', i === active));
}

function hideSuggestions() { sugg.classList.add('hidden'); active = -1; }

async function selectAddress(address) {
  q.value = address;
  hideSuggestions();
  result.innerHTML = '<p class="empty">Söker hämtningsinformation…</p>';
  try {
    const res = await fetch('/preview?address=' + encodeURIComponent(address));
    if (!res.ok) { result.innerHTML = '<p class="empty">Något gick fel.</p>'; return; }
    const data = await res.json();
    renderResult(address, data);
  } catch (e) {
    result.innerHTML = '<p class="empty">Något gick fel.</p>';
  }
}

function renderResult(address, data) {
  const keys = Object.keys(data);
  if (!keys.length) {
    result.innerHTML = '<p class="empty">Adressen hittades inte i hushållsregistret. ' +
      'Detta är vanligt för flerfamiljshus och samfälligheter.</p>';
    return;
  }
  const url = location.origin + '/ics?address=' + encodeURIComponent(address);
  const webcal = 'webcal://' + location.host + '/ics?address=' + encodeURIComponent(address);
  const gcalUrl = 'https://calendar.google.com/calendar/u/0/r?cid=' + encodeURIComponent(webcal);
  const outlookUrl = 'https://outlook.live.com/calendar/0/addfromweb?url=' +
                     encodeURIComponent(url) + '&name=' + encodeURIComponent('Sophämtning ' + address);

  let html = '<h2 style="font-size:1.1rem;margin:0 0 .5rem 0">' + escapeHtml(address) + '</h2>';
  html += '<div class="pickups">';
  for (const type of keys) {
    for (const e of data[type]) {
      html += '<div class="pickup"><span>' + escapeHtml(type) + ' — ' + escapeHtml(e.FetchFrequency) +
              '</span><b>' + escapeHtml(e.ExecutionDate) + ' (' + escapeHtml(e.Weekday) + ')</b></div>';
    }
  }
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
}

function copyUrl() {
  const el = document.getElementById('ics-url');
  el.select(); document.execCommand('copy');
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({ '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;' }[c]));
}
function escapeAttr(s) { return escapeHtml(s); }
</script>
</body>
</html>
"#;
