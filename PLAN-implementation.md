# Implementation plan — sopor coverage expansion (2026-09-12)

Konsoliderat från 8 parallella research-agenters fynd över alla ~206
oimplementerade svenska kommuner. Detaljerad research-rådata finns i
konversationshistoriken; detta är den handlingsbara sammanfattningen.

Startläge: 76 kommuner. Efter steg 1–9: **113 kommuner (39% av 290)**.

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

**Summa: +37 kommuner via 10 nya providers + 3 tenant-utökningar.**

## Steg 1 — Nya EDP FutureWeb-tenants (verifierade + implementerade)

| Kommun(er) | Tenant | api_url |
| --- | --- | --- |
| Ljusnarsberg, Lindesberg, Nora, Hällefors | SB Bergslagen | `futureweb.sbbergslagen.se/FutureWeb` |
| Falkenberg | Vivab / FutureWebFalken | `minasidor.vivab.info/FutureWebFalken` |
| Jönköping, Habo, Mullsjö | June Avfall / FutureWebJuneBasic | `minasidor.juneavfall.se/FutureWebJuneBasic` |
| Ludvika + Smedjebacken | WBAB | `futureweb.wbab.se/EDPFutureweb(Smedjebacken)` |
| Lidingö | Lidingö V&A | `vaochavfall.lidingo.se/Futureweb` |
| Lund | LRV | `eservice431601.lund.se/lund/FutureWeb` |
| Kramfors | Kramfors kommun | `futureweb.kramfors.se/EDPFutureWebBasic` |
| Svenljunga | Svenljunga kommun | `edpfutureweb.svenljunga.se/FutureWeb` |

**Hoppade tenants** (returnerade `Succeeded:true` men tomt dataset —
troligen icke-populerade instanser):
- Kristinehamn `varh.kristinehamn.se`
- Motala + Vadstena `vattenochavfall.motala.se`
- Trelleborg `kretsloppochvatten.trelleborg.se`

Värt separat probe senare — kanske de fyller på databasen.

## Nästa steg (för framtida sessions)

### Fortfarande obygg: area-based (~26 kommuner)

Statiska ruttlistor som Arjeplog/Arvidsjaur-mönstret. Varje kräver
manuell transkribering av veckodag + parity + områden från kommunens
sida (~30 min/styck):

Dorotea, Åsele, Ragunda, Valdemarsvik, Jokkmokk, Pajala, Vilhelmina,
Åmål, Norsjö, Sorsele, Övertorneå, Bjurholm, Degerfors, Filipstad,
Malå, Storuman, Vännäs, Årjäng, Färgelanda, Strömsund, Ydre,
Överkalix, Åre (Lundstams), Malung-Sälen (VAMAS), Öckerö (fram tills
Sysav-migrationen är klar).

### Fortfarande obygg: NÅRAB Indecta-variant

Klippan + Perstorp + Örkelljunga via `narabtomningskalender.se`.
Använder samma PHP-motor som webbservice.indecta.se men med:
- Extra fält i adress-datasetet (category-kolumn)
- Kalendermarkers som färgade inline `background:#...` istället för
  `dagMedTomClass<code>` CSS-klasser
- Fler URL-params i online_kalender-anropet (`knR`, `abK`, `clid`)

Kräver ny provider (kan inte återanvända befintlig `indecta.rs`).

### Fortfarande obygg: single-kommun candidates

- **Kristianstad** — Vue-widget på renhallningen-kristianstad.se; anropar
  troligen EDP men bakom minifierad Vue-app utan tydlig public endpoint.
- **HEMAB Härnösand** — SiteVision-search som kräver exakt gatunamn
  utan husnummer (usable-only för directly-known adress).
- **Karlskrona (Affärsverken)** — `/api/v1/open-api/*` kräver Bearer-token
  från login-flöde (BankID).

### Avfallsappen bearer-token research (låser upp ~30 kommuner)

Om bearer-flödet knäcks öppnas ~30 kommuner på en gång: Habo (redan
via June), Kinda, Sigtuna, Håbo, Tranemo, Kungsbacka, Kil, Sunne,
Hudiksvall, Finspång, Krokom, Söderhamn, Bollnäs+Ovanåker (BORAB),
Söderköping, Knivsta, Tyresö, Åtvidaberg, Lekeberg+Hallsberg+
Askersund+Laxå (sydnarke), Kalix, Vallentuna, Upplands-Bro, Tidaholm
m.fl.

### Sörmland Vatten (Katrineholm/Vingåker/Flen)

Skippad på grund av:
- admin-ajax kräver session-validerad nonce (server accepterar inte
  bara scraped nonce från annan session)
- Public data finns i attached xlsx-fil (attachment_id=18483) men
  kräver ny dep (`calamine` eller `zip`+xml) — inte värt för 3 kommuner
  om inte andra kommuner behöver xlsx-parsing.

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
- **YSBFU** (Ystad)
- **SATER** (Säter via Borlänge Energi)
- **VASTE** (VMEAB Västervik)
- Nordmaling, Vindeln — Vakin/UMEVA (redan känd)
- Alingsås Energi (redan känd)
- Karlstads Energi (redan känd)
- Skurup — `minasidor.skurup.se`
- Ystad — `pfu.ystad.se`

### BM FetchPlanner (utökad från AMAQ till NSR)
- **NSR** (Bjuv, Åstorp, Båstad, Helsingborg, Höganäs, Ängelholm) — 6 kommuner. Frontend Next.js SPA på `nsr.se`/`minasidor.nsr.se` men backend `fpmobile.nsr.se/FetchPlannerService/CustomerAccountHandler.svc` är BankID-gated.

### Andra login-only backends
- **EDP FutureWebBasic (LOGIN-variant)**: Östersund, Värmdö, Luleå exponerar bara `/FutureWebID/` eller `/FutureWebBasic/`-utan-`SimpleWastePickup`.
- **VIVAB (Varberg)**: EXDE-liknande Java-portal, login-gated. **Falkenberg** däremot går via `FutureWebFalken`-varianten (öppen — implementerad).
- **Rambo Mina sidor** (`minasidor.rambo.se`): BankID-gated — MEN `rambo.se/wp-json/app/v1/` är anonymt (implementerad).
- **DVA/Nodava** (Leksand, Mora, Älvdalen, Gagnef, Rättvik, Vansbro, Orsa): "Mitt DVA"-app + `minasidor.nodava.se`, alla BankID.
- **Nordic Peak Open ePlatform**: Emmaboda, Gotland, Härryda, Mjölby, Nora (etjanster.nora.se är separat från SB Bergslagen), Övertorneå — alla BankID.
- **MERAB** (Eslöv, Höör, Hörby): Gatsby+EDP shim `/buildings` kräver `edpCustomer` (post-BankID). Anonym `/api/edp/buildings/search?q=` returnerar permanent tomt.
- **Karlskrona (Affärsverken)** — Bearer-token från `/api/v1/open-api/login`.
