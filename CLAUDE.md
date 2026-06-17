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
  main.rs              Axum routing, handlers. Routes are /:kommun/{autocomplete,preview,ics}.
  templates.rs         Inline-rendered HTML (no template engine). render_index + render_kommun.
  ical.rs              VCALENDAR generation. Two emission modes (see below).
  providers/
    mod.rs             Provider trait, normalized types, Registry::build (where new kommuner get wired in).
    stockholm.rs       SVOA custom JSON.
    sitevision_fetchplanner.rs   Limepark widget; multi-tenant by Config.
    vasyd.rs           Malmö+Burlöv; open POST JSON; multi-tenant by city allow-list.
    indecta.rs         OGRAB+Sjöbo; PHP/Latin-1; HTML grid parser.
```

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

`cargo test --quiet` should always pass. Currently 27 tests; add
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
- **EDP Future / FutureWeb (VertiGIS)** — biggest unexplored target
  (~100 kommuner including Luleå, Skellefteå, Uppsala, Roslagsvatten
  region, Sörmland Vatten, Vakin). Each instance has its own subdomain
  and the entry point is a `/EDPLogin/LogIn`-fronted iframe. Not yet
  built. This is the next high-leverage target if expansion continues.
- **Open data portals (dataportal.se, kommun ArcGIS hubs)** — checked
  exhaustively. Zero of 290 kommuner publish address-based pickup
  schedules as authorized open data. All current adapters are
  scrape-based by necessity.

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
