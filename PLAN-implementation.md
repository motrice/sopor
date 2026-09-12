# Implementation plan — sopor coverage expansion (2026-09-12)

Konsoliderat från 8 parallella research-agenters fynd över alla ~206
oimplementerade svenska kommuner.

Startläge: 76 kommuner. Efter alla implementerade steg: **133 kommuner (46% av 290).**

## Status per steg

| Steg | Provider(s) | Kommuner | Status | Commit |
| --- | --- | --- | --- | --- |
| 1 | EDP FutureWeb-tenants ×11 | +14 | ✅ | `95c3c99` |
| 2 | Indecta SÅM | +5 | ✅ | `bcf11fd` |
| 3 | Rambo WP-JSON | +4 | ✅ | `a0fb21c` |
| 4 | Sysav Azure EDP-proxy | +3 | ✅ | `66a5258` |
| 5 | LSR öppet REST | +2 | ✅ | `29c65e8` |
| 6 | VMR (Härjedalen/Berg/Bräcke) | +3 | ✅ | `dd4b614` |
| 7a | Sörmland Vatten | 0 | ⛔ skipped (nonce+xlsx dep) | — |
| 7b | Optigon Avfallskollen | +3 | ✅ | `15ddf99` |
| 7c | MERAB Gatsby+EDP | 0 | ⛔ skipped (auth-gated) | — |
| 9 | Strömstad + VMAB Bromölla + Ronneby | +3 | ✅ | `9496dee` |
| 8/10 | area_based ×19 kommuner | +19 | ✅ | `c70872f` |
| 11 | HEMAB Härnösand | +1 | ✅ | `a296c72` |

**Summa: +57 kommuner via 12 nya providers + 3 tenant-utökningar.**

## Kvarstår att bygga

Det som återstår är alla höga-tröskel eller blockerade fall.
Sorterat efter ROI (avkastning per timmes arbete):

### Hög ROI om det går: Avfallsappen bearer-token (~30 kommuner)

Om bearer-flödet mot `<tenant>.avfallsapp.se`-widgeten knäcks öppnas
30+ kommuner på en gång. Bekräftade Bozzanova-tenants med confirmed
real-data-svar:

- **AÅS (Skaraborg)** — redan täckt via egen aas.rs
- **BORAB** — Bollnäs + Ovanåker
- **June** (Habo) — redan täckt via EDP
- **rambo** — Lysekil/Sotenäs/Munkedal/Tanum (redan täckt via WP-JSON)
- **sydnarke** — Lekeberg + Hallsberg + Askersund + Laxå

Övriga confirmed-real tenants (30-ish nya kommuner):
Habo (duplicate), Kinda, Sigtuna, Håbo, Tranemo, Kungsbacka, Kil,
Sunne, Hudiksvall, Finspång, Krokom, Söderhamn, Söderköping, Knivsta,
Tyresö, Åtvidaberg, Kalix, Vallentuna, Upplands-Bro, Tidaholm m.fl.

**Blocker:** widget-JS-bundle innehåller inte bearer-token i klar-text.
Token verkar plockas efter ett register/bind-flöde. HACS-implementation
finns för Home Assistant men okänt om den knäcker anonymt läge eller
kräver användarens BankID-inloggning.

### Låg ROI men rimlig komplexitet: single-kommun (~4 kommuner)

- **Kristianstad** Vue-widget (`renhallningen-kristianstad.se`) —
  **verifierat trasig 2026-09-12**: Vue-mallen finns i HTML:en med
  `v-model="calendarValue"`, `@click="calendarPickAdress(result)"`
  etc, men *ingen* Vue-komponent med dessa metoder är laddad — bara
  `#sort-guide`- och `#faq`-Vue-apparna finns i script.js. WP-JSON
  `kr/`-namespace innehåller bara `kr/centrals` (återvinningscentraler).
  Widgeten är deployad utan sitt backend-JS. Ingenting att skrapa.
- ~~**HEMAB Härnösand**~~ — implementerad `a296c72`: `?query=*`
  returnerar hela datasetet (1790 adresser). Cachas 12 h och
  substring-matchas lokalt.
- **NÅRAB** (Klippan/Perstorp/Örkelljunga) — **verifierat 2026-09-12
  som svårare än research-agenten uppskattade**: PHP-endpointen
  `online_kalender_skapa.php` returnerar ett tomt kalender-template
  utan pickup-datum. Datumen genereras klient-side via minifierad JS
  när användaren klickar en PDF-knapp. Skulle kräva reverse
  engineering av `jq.motor.js`-baserad PDF-generator (flera hundra
  rader minifierad JS). Inte samma svårighet som OGRAB som har
  server-renderade `dagMedTomClass`-markers.

### Låg ROI, hög komplexitet: PDF/OCR/xlsx-parsing (~7 kommuner)

Alla dessa har publik data men i format som skulle kräva nya dependencies:

- **Malung-Sälen (VAMAS)** — 21 text-PDFer, en per område. Skulle
  kräva `pdf-extract` eller `lopdf` dep.
- **Överkalix** — mixad 3-veckors-vinter + 2-veckors-sommar
  publicerat som text-PDF med explicita datum per område. Kräver
  PDF-parsing + explicit-datum-modell i `area_based`.
- **Färgelanda** — månads/kvartalsabonnemang som veckonummerlistor
  (`v.4,8,12,…`). Skulle kräva utökning av `area_based::Route` med
  `weeks: &[u32]`-varianten.
- **Öckerö** — 10 PDFer (en per ö), per-adress → veckodag-mapping
  med kommunens paritetsregel (mat=jämn, rest=udda). Bryter från
  route-slinga-mönstret. Kräver adress-lookup UI.
- **Sörmland Vatten** — publik `.xlsx` (Katrineholm+Vingåker+Flen),
  kräver `calamine` eller `zip`+xml.

### Blockerat: Avstår helt (~50 kommuner)

- **CGI BFUS** (~20 kommuner) — se dead-end-lista.
- **BM FetchPlanner** (~8 kommuner) — NSR, AMAQ etc.
- **DVA/Nodava** (~7 kommuner) — Mitt DVA-app + BankID.
- **Open ePlatform / Nordic Peak** (~6 kommuner) — BankID.
- **MERAB** (Eslöv/Höör/Hörby) — `/buildings` kräver `edpCustomer`.
- **Karlskrona (Affärsverken)** — Bearer via `/api/v1/open-api/login`.
- **EDP FutureWebBasic login-variant** (Östersund, Värmdö, Luleå) —
  ingen `SimpleWastePickup`-endpoint alls, bara `/FutureWebID/`.
- **Åmål** — kommunen publicerar inget schema.
- **Degerfors** — restavfall/matavfall aldrig publicerat, bara FNI.

## Realistiska mål framåt

Om Avfallsappen-flödet knäcks: **132 → ~162 kommuner (56%)**.

Även med all resterande low-ROI/high-complexity-arbete klart:
**132 → ~145 kommuner (50%)**.

Bortom 50% skulle kräva:
- Kontakt med kommunerna för att få dem publicera schemat (många
  kommuner har schemat internt hos entreprenören men publicerar inte).
- BankID/Freja-integration (utanför projektets scope idag).
- Kommun-drivna öppna-data-initiativ (Sundsvall är det enda goda
  exemplet idag).

## Bekräftade dead-ends (uppdatera CLAUDE.md separat)

### CGI BFUS (nya tenants bekräftade av research)
Utöver de sedan tidigare kända (STOVA, UMEVA, KARLS, ALING):
- **SEOM** (Sollentuna) — `/Environments/SOLLE/`
- **EEM** (Eskilstuna) — `pfu_lang`
- **GASAV** (Gästrike Återvinnare — Gävle, Hofors, Sandviken, Ockelbo, Älvkarleby)
- **BORLA** (Borlänge Energi)
- **EKSJO** (Eksjö Energi)
- **UDDEV** (Uddevalla Energi)
- **ULRIC** (UEAB Ulricehamn)
- **YSBFU** (Ystad) — `pfu.ystad.se`
- **SATER** (Säter via Borlänge Energi)
- **VASTE** (VMEAB Västervik)
- Skurup — `minasidor.skurup.se`
- Nordmaling, Vindeln — Vakin/UMEVA (redan känd)

### BM FetchPlanner (utökad från AMAQ till NSR)
- **NSR** (Bjuv, Åstorp, Båstad, Helsingborg, Höganäs, Ängelholm) —
  6 kommuner. Frontend Next.js SPA på `nsr.se`/`minasidor.nsr.se` men
  backend `fpmobile.nsr.se/FetchPlannerService/CustomerAccountHandler.svc`
  är BankID-gated.

### Andra login-only backends
- **EDP FutureWebBasic (LOGIN-variant)**: Östersund, Värmdö, Luleå
  exponerar bara `/FutureWebID/` eller `/FutureWebBasic/`-utan-
  `SimpleWastePickup`.
- **VIVAB (Varberg)**: EXDE-liknande Java-portal. **Falkenberg**
  däremot via `FutureWebFalken`-varianten (öppen — implementerad).
- **Rambo Mina sidor** (`minasidor.rambo.se`): BankID-gated — MEN
  `rambo.se/wp-json/app/v1/` är anonymt (implementerad).
- **DVA/Nodava** (Leksand, Mora, Älvdalen, Gagnef, Rättvik, Vansbro, Orsa):
  BankID.
- **Nordic Peak Open ePlatform**: Emmaboda, Gotland, Härryda, Mjölby,
  Nora (etjanster.nora.se är separat från SB Bergslagen), Övertorneå
  (huvud-e-tjänsten; själva sophämtningsschemat är dock ren HTML —
  därför implementerad via area_based).
- **MERAB** (Eslöv, Höör, Hörby): Gatsby+EDP-shim `/buildings` kräver
  `edpCustomer` (post-BankID). Anonym `/api/edp/buildings/search?q=`
  returnerar permanent tomt.
- **Karlskrona (Affärsverken)** — Bearer-token från
  `/api/v1/open-api/login`.

## Data-format-utmaningar (för framtida sessions)

Dessa kommuner har data offentligt men i format som skulle kräva nya
dep eller större refactorer:

- **Färgelanda** — `area_based::Route` behöver `weeks: &[u32]`-variant
  för månads/kvartalsabonnemang.
- **Överkalix** — behöver `dates: &[NaiveDate]`-variant för mixade
  frekvenser (3v vinter / 2v sommar).
- **Malung-Sälen** — 21 text-PDFer, kräver PDF-parsing.
- **Sörmland Vatten** — `.xlsx`, kräver zip+xml eller calamine.
- **Öckerö** — 10 PDFer + per-adress-lookup, bryter mot slinga-modellen.
