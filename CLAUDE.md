# Architecture notes (for Claude)

This file is loaded into every Claude session in this repo. It captures
context that isn't obvious from reading the code alone.

## What this is

A Rust + Axum web service that turns Swedish municipal "när kommer
sopbilen" address lookups into iCalendar (`text/calendar`) subscriptions.
Each kommun has its own URL slug (`/stockholm`, `/falun`, …). Users
subscribe in Google Calendar / Apple Calendar / Outlook; the feed
refreshes itself on the client's schedule (every ~12 h is typical).

Project goal: cover as many of Sweden's 290 kommuner as feasible by
clustering them by *backend platform*, not by kommun. README has the
running coverage table.

## Source tree

```
src/
  main.rs              Axum routing, handlers, swedish_sort_key for landing list.
  templates.rs         Inline-rendered HTML (no template engine). render_index + render_kommun.
  ical.rs              VCALENDAR generation. Two emission modes (see below).
  providers/
    mod.rs             Provider trait, normalized types, Registry::build (where new kommuner get wired in).
    stockholm.rs       SVOA custom JSON.
    sitevision_fetchplanner.rs   Limepark widget (Falun, Örnsköldsvik); multi-tenant by Config.
    vasyd.rs           Malmö+Burlöv; open POST JSON; multi-tenant by city allow-list.
    indecta.rs         OGRAB (Östra Göinge, Osby) + Sjöbo; PHP 5.4/Latin-1; HTML grid parser.
    edp_future.rs      Skellefteå + 42 more (Vafab, SSAM, Kretslopp Sydost, Remondis, NVOA, etc.).
                       Open POST JSON; needs explicit Content-Length: 0 header on IIS.
    roslagsvatten.rs   Ekerö, Vaxholm, Österåker. Drupal AJAX-array with embedded HTML fragments.
```

Current coverage: 54 kommun-routes (alphabetically sorted on landing, Swedish å<ä<ö order).

## Core abstractions

- **`Provider` trait** (`async_trait`, `Send + Sync`) — `id`, `name`,
  `placeholder`, `note`, `autocomplete`, `schedule`.
- **`Suggestion { value }`** — string the user picks.
- **`PickupSchedule { address, series: Vec<PickupSeries> }`**
- **`PickupSeries { waste_type, frequency_text, interval_weeks, anchor: Vec<NaiveDate> }`**

The `anchor` field is overloaded by design:
- `interval_weeks = Some(N)` + `anchor.len() == 1` → ical.rs emits one
  VEVENT with `RRULE:FREQ=WEEKLY;INTERVAL=N;COUNT=…`.
- Otherwise → one explicit VEVENT per anchor date, no RRULE.

This lets providers that only return "the next pickup" (Stockholm) get
RRULE projection, while providers that return an explicit upcoming list
(Falun, Indecta, VA SYD's non-weekly types) emit literal dates and rely
on calendar refresh.

## Adding a kommun

The default path: figure out which **platform** the kommun uses, then
either reuse an existing provider with a new `Config`, or add a new
provider module. Update `Registry::build` and the README table.

Existing platforms with multi-tenant configs:
- `SitevisionFetchplanner` (Falun, Örnsköldsvik) — for sites with
  `class="sv-limepark-app-fetchplanner"` or `…-webapp-fetchplanner`.
- `VaSyd` (Malmö, Burlöv) — VA SYD's POST JSON API on `vasyd.se`.
- `Indecta` (Östra Göinge, Osby, Sjöbo) — for sites embedding the
  `webbservice.indecta.se/kunder/<client>/` iframe.
- `EdpFuture` (43 kommuner) — for any deployment of EDP
  `/FutureWeb/SimpleWastePickup` (also `EDPFutureWeb`, `FutureWebBasic`,
  `FutureWebOS`, etc.). One Config per kommun-route; multi-kommun bolag
  (SSAM, Vafab Miljö, Kretslopp Sydost, Remondis) use a `cities`
  allow-list to filter the shared upstream.
- `Roslagsvatten` (Ekerö, Vaxholm, Österåker) — `/schedule/search` +
  `/schedule/fetch` POST JSON, returns Drupal AJAX-array.

When adding via existing provider, all you usually need is a `Config`
literal in `Registry::build`. New platform = new file under
`providers/`, implement `Provider`, register.

## Provider encoding/parsing gotchas

- **Stockholm (SVOA)** — straightforward GET JSON. PascalCase fields.
- **SitevisionFetchplanner** — page embeds
  `AppRegistry.registerInitialState('<portlet>', {…})`. We find the
  call by literal substring, then let `serde_json::Deserializer` consume
  the leading JSON value (it stops at the matching `}`). Two response
  shapes coexist: `hits[]` (ambiguous query, many Calendars per
  container) vs `containers[]`/`trips[]` (resolved). Miva variant has
  no `hasCalendars` and ISO dates live on `trips[]` keyed by `typeText`.
  Times are UTC; convert to `Europe/Stockholm` via `chrono-tz` before
  taking the date — UTC `22:00:00Z` in summer is the *next* local day.
- **VA SYD** — POST `application/json`, body `{"query": …}` then
  `{"query": <id>, "street": <full>}`. Returns near-future dates (1–2)
  with a human-readable frequency string. `"Onsdag varje vecka"` parses
  to weekly RRULE; `"Måndag, torsdag varje vecka"` does NOT — a single
  weekly anchor would miss the alternate day, so emit explicit dates.
- **Indecta** — PHP 5.4 era. Outbound URLs MUST be ISO-8859-1
  percent-encoded (`Å` = `%C5`). Inbound encoding sniffed: try UTF-8,
  fall back to Latin-1. OGRAB returns 4-field rows
  (`street|city|kundnr|anlnr`); Sjöbo returns 2-field rows and the
  calendar call works without `nrA`. Address API also has variants
  across deployments — keep the parser permissive. HTML calendar is
  a 12-month grid; we walk months by `styleMonthName` headers and pick
  up days via the `class="styleDayHit"` + `dagMedTomClass<code>` pair.
- **EdpFuture** — IIS returns 411 Length Required if the POST has
  empty body without `Content-Length: 0` header (reqwest defaults to
  chunked transfer encoding for empty bodies). Always set it explicitly.
  Building strings (e.g. `"Frögatan 76 -150, SKELLEFTEÅ (133427)"`)
  round-trip verbatim — feed them back to GetWastePickupSchedule as-is.
  Three date formats coexist in `NextWastePickup`: ISO `YYYY-MM-DD`,
  ISO week `v25 Jun 2026`, and month-only `Jun 2026`. Frequency is
  derived primarily from `WastePickupsPerYear` (52/26/13 → weekly/
  biweekly/4-weekly); higher cadences and any frequency text containing
  a comma (multi-day weekly) emit explicit single dates rather than
  incorrect RRULE projections. Swedish-character matching in city
  allow-lists requires Unicode `.to_lowercase()`, not
  `eq_ignore_ascii_case`.
- **Roslagsvatten** — Drupal AJAX. The endpoints return a JSON array
  of `{command, method, selector, data}` where `data` is an HTML
  fragment string. Extract addresses via regex on `data-bid="ID"`+text;
  extract schedule entries via `<h3>type</h3>` + `Frekvens: ...` +
  `Nästa hämtning: YYYY-MM-DD` within each `<div class="waste-schedule-inner">`.
  Search endpoint returns HTTP 500 for queries < 3 characters — mirror
  the upstream's 3-char Drupal debounce in our autocomplete or upstream
  errors out. `"udda vecka"` / `"jämn vecka"` both map to biweekly.

## ical.rs conventions

- `X-WR-CALNAME: Sophämtning - <address>` per feed.
- All events all-day (`DTSTART;VALUE=DATE`).
- `VALARM` 6 hours before midnight (so notification fires ~18:00 the
  day before pickup). Apple Calendar respects subscribed `VALARM`,
  Google Calendar on Android does NOT — known limitation, documented
  in README.
- `UID` is `sha256(kommun | address | waste_type [| date])[..24] + "@sopor.motrice.se"`.
  Including `kommun` in the UID prevents collisions across kommuner.

## Testing

Inline `#[cfg(test)] mod tests` per provider. No network in tests. To
make scraping testable, each provider's `schedule()` is split: the HTTP
fetch stays in the `Provider` impl, the parsing/building logic is a
free function (`parse_schedule`, `build_schedule`, `parse_calendar`)
that takes a string/struct and returns `PickupSchedule`. Fixtures are
inline string literals — small enough to keep readable.

`cargo test --quiet` should always pass. Currently 46 tests; add
specific cases for new edge cases discovered while implementing a
provider rather than scaling test count for its own sake.

## Conventions

- **Swedish** is the user-facing language. UI copy, kommun names,
  waste-type labels — all Swedish. Code identifiers, comments, and PR
  messages stay in English.
- **License: GPL-3.0-only** (not `-or-later`; the user dislikes the
  blank-check semantics of `-or-later`).
- **Commit messages**: conventional commits (`feat:`, `fix:`, `test:`,
  `refactor:`, `ci:`, `docs:`). The user signs every commit with a
  YubiKey afterwards (`git commit --amend --no-edit -S`); do NOT add a
  `Co-Authored-By` trailer — this conflicts with the signing identity.
- **Single-line comments only** in `.rs` source. Use them sparingly for
  *why*, not *what*.
- **No new deps without good reason.** Current deps are intentionally
  minimal: axum, tokio, reqwest, serde, chrono(+tz), sha2,
  percent-encoding, tracing, async-trait, regex. Don't pull in
  `scraper`, `html5ever`, `anyhow`, etc., without discussing.
- **Sort kommun lists in Swedish alphabetical order** (a–z, then å, ä, ö).
  Use `swedish_sort_key()` in `main.rs` rather than plain `.to_lowercase()`
  which gives Unicode order ä < å.

## Things tried and rejected

- **Apple Påminnelser (Reminders) integration** — there is no
  `webcal://` equivalent for Reminders. Subscribed iCal feeds with
  `VTODO` are not consumed by Reminders.app; only CalDAV provides
  ongoing sync. We rejected a one-shot `.ics` import button as
  confusing UX and reverted the code in `5afc475`'s prior history.
- **Avfallsappen (Bozzanova)** — ~50 kommuner. API exists at
  `<kommun>.avfallsapp.se/(wp-json|api)/nova/v1/` but pickup lookup
  requires a Bearer token extracted from the mobile app + a
  register/bind dance. Schema drifts per tenant. Decided against until
  there's a clear authorization story. HACS reference impl exists at
  `mampfes/hacs_waste_collection_schedule`.
- **Open data portals (dataportal.se, kommun ArcGIS hubs)** — checked
  exhaustively. Zero of 290 kommuner publish address-based pickup
  schedules as authorized open data. All current adapters are
  scrape-based by necessity.
- **CGI BFUS (Business For Utilities Suite)** — the Mina sidor /
  customer portal layer of CGI's BFUS product. Product page:
  `https://www.cgi.com/se/sv/business-for-utilities-suite` (says 70+
  customer companies, none listed publicly). The portal is
  ASP.NET WebForms + Knockout.js + jQuery, with WSDL/Swagger/OpenAPI
  intentionally not exposed.

  Confirmed customers:
  - Vakin / UMEVA (Umeå, Vindeln, Nordmaling) — `minasidor.vakin.se`,
    `/Environments/UMEVA/`
  - Stockholm Vatten och Avfall — `pfu.stockholmvattenochavfall.se/
    flex/flexservices.aspx` (visible as the "Mina sidor"-link in SVOA's
    frontend, see session 1), `/Environments/STOVA/`
  - Karlstads Energi — `minasidor.karlstadsenergi.se`,
    `/Environments/KARLS/`

  Identification fingerprints (any one is sufficient):
  - `Portal-Version` meta with `CGI.Utility.Application.CPU.Client.Web.dll`
  - `pfu_lang` cookie set on first response
  - `/Environments/<CLIENT_SLUG>/` paths in stylesheets

  Verified BankID/login-gated end-to-end (session 2):
  - Probed standard public paths (`/api/StartUp`, `/api/Tomning`,
    `/api/Schedule`, `/swagger`, `/openapi`, `/Services/PFU.asmx?WSDL`,
    `/Customer/*`, `/api/Public/*`, `/api/Anonym/*`) — all 404 or empty
    catch-all 200.
  - The only app-specific paths leaking from `site.min.js`
    (`/api/grp2/`, `/api/subuser/`, `/api/swish/`, `/api/grp2/GenerateQr/`)
    all return 404 without auth.
  - CGI's product page mentions "standardiserade integrationer via
    API:er" but only for internal case-creation — no public developer
    docs, no third-party widgets.

  **Don't re-probe.** Skip CGI BFUS entirely. Re-attempt only if
  (a) a kommun publishes a separate non-BFUS widget, (b) we build
  BankID/Freja auth support (separate project, brushes against
  personuppgifter), or (c) a HACS source reverses the post-login
  endpoints. Direct contact: Lasse Andersson, lar.andersson@cgi.com
  (BFUS product manager).
- **MSVA — Mittsverige Vatten & Avfall (Sundsvall, Timrå, Nordanstig)** —
  investigated, *not implemented*. The widget on `msva.se` is a custom
  SiteVision React app (`sv-garbageScheduleExtended`) that fetches via
  `getUrl("/allAddresses")`. The actual REST URL path is determined by
  the SiteVision SDK at runtime from the deployed app's registered
  name, which I could not discover by enumeration (probed
  `/rest-api/<webapp-id>`, `/rest-api/garbageScheduleExtended`,
  `/rest-api/sv-garbageScheduleExtended`, several plausible app-name
  guesses — all return `{"success":false,"type":"invalidParameter",
  "message":"No RestApp found for ..."}`). The HACS reference uses
  `https://api.sundsvall.se/Garbage/2281/schedules?street=...&
  houseNumber=...&postalCode=...&city=...` directly, which works but
  requires the user to enter postal code (and only covers kommunkod
  2281 = Sundsvall, not the other MSVA members). Re-attempt either by:
  (a) finding a postal-code-from-street source for Sundsvall and
  proxying via api.sundsvall.se, (b) building a special form variant
  for MSVA that asks for postal code explicitly, or (c) discovering
  the SiteVision RestApp name via either a SiteVision admin login or
  observing the actual XHR via a browser DevTools session.

## Deployment

Multi-stage `Dockerfile` (rust:1.95-bookworm → debian:bookworm-slim,
non-root user 10001). `PORT` env var (default 8080), `RUST_LOG`
(default `info`). GitHub Actions in `.github/workflows/docker.yml`
builds linux/amd64 + linux/arm64, pushes to GHCR on `main` and tags,
attaches SLSA provenance + SBOM.

## Working with the user

- The user is Björn (bjorn.molin@motrice.se), in Sweden. Replies in
  Swedish when the user writes Swedish; otherwise English is fine.
- The user prefers honest assessments over premature implementation.
  When a task is large, *report findings first* (HACS exists, EDP is
  complex, Avfallsappen needs auth) before starting to build.
- The user accepts brittleness for legacy systems where it's clearly
  worth it (Indecta was built knowing PHP 5.4 backends rarely change).
- Background command failures with exit code 143 after `kill` are
  expected — that's just the SIGTERM after the smoke test.
