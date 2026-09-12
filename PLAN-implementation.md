# Implementation plan — sopor coverage expansion (2026-09-12)

Konsoliderat från 8 parallella research-agenters fynd över alla ~206
oimplementerade svenska kommuner. Detaljerad research-rådata finns i
konversationshistoriken; detta är den handlingsbara sammanfattningen.

Nuvarande täckning: 76 kommuner (26% av 290).

## Step 1 — Nya EDP FutureWeb-tenants (~18 kommuner)

Bara `Config`-tillägg i `providers/mod.rs` mot befintlig `edp_future`.
Verifiera först att `/SearchAdress?searchText=…` svarar för varje URL.

| Kommun(er) | Tenant | api_url |
| --- | --- | --- |
| Kristinehamn | Kristinehamns kommun | `https://varh.kristinehamn.se/FutureWeb/SimpleWastePickup` |
| Ljusnarsberg, Lindesberg, Nora, Hällefors | Samhällsbyggnad Bergslagen | `https://futureweb.sbbergslagen.se/FutureWeb/SimpleWastePickup` |
| Motala, Vadstena | MVVAB / Motala | `https://vattenochavfall.motala.se/FutureWeb/SimpleWastePickup` |
| Falkenberg | Vivab / FutureWebFalken | `https://minasidor.vivab.info/FutureWebFalken/SimpleWastePickup` |
| Jönköping, Habo, Mullsjö | June Avfall / FutureWebJuneBasic | `https://minasidor.juneavfall.se/FutureWebJuneBasic/SimpleWastePickup` |
| Trelleborg | Kretslopp och vatten | `https://kretsloppochvatten.trelleborg.se/EDPFutureweb/SimpleWastePickup` |
| Ludvika, Smedjebacken | WBAB | `https://futureweb.wbab.se/EDPFutureweb/SimpleWastePickup` (+`/EDPFuturewebSmedjebacken/…`) |
| Lidingö | Lidingö V&A | `https://vaochavfall.lidingo.se/Futureweb/SimpleWastePickup` |
| Lund | Lunds Renhållningsverk | `https://eservice431601.lund.se/lund/FutureWeb/SimpleWastePickup` |
| Kramfors | Kramfors kommun | `https://futureweb.kramfors.se/EDPFutureWebBasic/SimpleWastePickup` |
| Svenljunga | Svenljunga kommun | `https://edpfutureweb.svenljunga.se/FutureWeb/SimpleWastePickup` |

Multi-kommun-tenanter (SB Bergslagen, Motala/MVVAB, June, WBAB) delas
via `cities`-allowlist precis som Vafab.

## Step 2 — Ny Indecta-tenant (+5 kommuner)

`Config` mot `webbservice.indecta.se/kunder/sam/` för SÅM Samverkan
Återvinning Miljö.

- **Kommuner**: Gislaved, Gnosjö, Hylte, Vaggeryd, Värnamo.
- **Not**: Samma iframe-flöde som OGRAB/Sjöbo. Verifiera adress-encoding
  (ISO-8859-1?) och om HTML-grid-parsern matchar.

## Step 3 — Ny provider: Rambo (WP-JSON, statisk UUID) (+4 kommuner)

- **Kommuner**: Lysekil, Munkedal, Sotenäs, Tanum.
- **Endpoint**: `https://rambo.se/wp-json/app/v1/{address-flat,next-pickup-web}`.
- **Auth**: statisk `X-App-Identifier: 202d6cd8-389c-4ab4-8921-7183378eb477` (från appen).
- **Filstruktur**: `providers/rambo.rs` — GET-baserad, single-tenant.
- **Verify först**: `curl -H "X-App-Identifier: <uuid>" https://rambo.se/wp-json/app/v1/address-flat?q=Storgatan` — bekräfta anonymt.

## Step 4 — Ny provider: Sysav Azure EDP-proxy (~6 nya kommuner)

- **Nya kommuner**: Kävlinge, Lomma, Staffanstorp, Svedala, Öckerö (osäker), och några till om de inte är CGI BFUS.
- **Endpoint bas**:
  `https://ca-swec-sysav-public-edp-prod.bluedune-a5ae63ed.swedencentral.azurecontainerapps.io/api/PickupSchedules/{findbuilding,foraddress}/`
- **Auth**: anonym (proxy runt EDP FutureWeb för Sysav-ägare).
- **Filstruktur**: `providers/sysav.rs` — REST-baserad, multi-tenant med
  `cities`-filter.
- **Verify först**: `curl` mot `findbuilding/?query=…` och `foraddress/…`.

## Step 5 — Ny provider: LSR (öppet REST-API) (+2 kommuner)

- **Kommuner**: Landskrona, Svalöv.
- **Endpoints**:
  - `POST https://minasidor.lsr.nu/api/api/external/autocompleteAllPost/` (form-urlencoded)
  - `POST https://minasidor.lsr.nu/api/api/external/schedulePost/`
- **Auth**: ingen.
- **Filstruktur**: `providers/lsr.rs` — POST-baserad, multi-tenant.
- **Verify först**: `curl -X POST -d "query=Storgatan" …/autocompleteAllPost/`.

## Steg efter dessa 5 (för senare)

- **Step 6**: Härjedalen SiteVision `/allAddresses` (+ ev. Berg/Bräcke).
- **Step 7**: Sörmland Vatten (+3), MERAB Gatsby+EDP (+3), Optigon Avfallskollen (+3).
- **Step 8**: ~26 area-based kommuner (varje kräver manuell transkribering av slinga+veckodag+parity från kommunens sida).
- **Step 9**: Enskilda single-kommun (Strömstad, VMAB Bromölla, Ronneby, Kristianstad, HEMAB, Karlskrona, NÅRAB Klippan/Perstorp/Örkelljunga).
- **Step 10**: Avfallsappen bearer-token research (låser upp ~30 kommuner om det går).

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

### BM FetchPlanner (utökad från AMAQ till NSR)
- **NSR** (Bjuv, Åstorp, Båstad, Helsingborg, Höganäs, Ängelholm) — 6 kommuner. Frontend Next.js SPA på `nsr.se`/`minasidor.nsr.se` men backend `fpmobile.nsr.se/FetchPlannerService/CustomerAccountHandler.svc` är BankID-gated. Publik `tomningskalender`-widget existerar men läser data via session-gated bundle.

### Andra login-only backends
- **EDP FutureWebBasic (LOGIN-variant)**: Östersund, Värmdö, Luleå exponerar bara `/FutureWebID/` eller `/FutureWebBasic/`-utan-`SimpleWastePickup`. Skiljer sig från våra 43 anonymt-öppna EDP-tenants.
- **VIVAB (Varberg)**: EXDE-liknande Java-portal, login-gated. **Falkenberg** däremot går via `FutureWebFalken`-varianten (öppen).
- **Rambo Mina sidor** (`minasidor.rambo.se`): BankID-gated — MEN `rambo.se/wp-json/app/v1/` är anonymt (se Step 3).
- **DVA/Nodava** (Leksand, Mora, Älvdalen, Gagnef, Rättvik, Vansbro, Orsa): "Mitt DVA"-app + `minasidor.nodava.se`, alla BankID.
- **Nordic Peak Open ePlatform**: Emmaboda, Gotland, Härryda (tjanster.harryda.se), Mjölby, Nora (etjanster.nora.se), Övertorneå — alla BankID.
