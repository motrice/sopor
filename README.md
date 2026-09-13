# sopor

Kalenderprenumeration (iCalendar) för sophämtning i svenska kommuner.

**Detta är en demonstrationsapplikation** — den finns för att påvisa
vikten av **öppna data** och **öppna standarder** i den offentliga
sektorn. Av Sveriges 290 kommuner publicerar exakt en (Sundsvall)
hämtningsscheman som riktig öppen data. För övriga behöver den här
tjänsten scrapea proprietära widgets, sniffa legacy-encodingar och
reverse engineera JSON-nycklar — arbete som borde vara helt onödigt.
Läs [pitchen](https://sopkalender.se/pitch) för hela resonemanget.

**Licens och förvaltning.** Koden är fri och öppen under GPL-3.0-only
och får användas fritt inom licensens ramar. Jag driver dock inte det
här som ett förvaltat projekt — förvänta dig ingen löpande support,
felrättning eller anpassning för nya kommuner utan förfrågan. Om din
organisation vill realisera lösningen på riktigt (produktionssättning,
utökad kommun-täckning, förvaltningsåtagande) tar jag gärna sådana
uppdrag — hör av dig via GitHub eller på
[LinkedIn](https://www.linkedin.com/in/bj%C3%B6rn-molin-1843aa3).

Backend (Rust + Axum) hämtar data live från respektive kommuns offentliga
adressuppslagstjänst, och returnerar en `text/calendar`-feed med
återkommande events (RRULE) eller explicit listade datum, plus VALARM 6
timmar före (notis ~18:00 dagen innan på Apple Calendar).

## Stödda kommuner

| Kommun | URL | Plattform |
| --- | --- | --- |
| Stockholm | `/stockholm` | Stockholm Vatten och Avfall (custom) |
| Falun | `/falun` | SiteVision FetchPlanner (Limepark) via Falu Energi & Vatten |
| Örnsköldsvik | `/ornskoldsvik` | SiteVision FetchPlanner (Limepark) via Miva |
| Malmö | `/malmo` | VA SYD (öppet POST JSON API) |
| Burlöv | `/burlov` | VA SYD (öppet POST JSON API) |
| Östra Göinge | `/ostra-goinge` | Indecta OnlineKalender via OGRAB |
| Osby | `/osby` | Indecta OnlineKalender via OGRAB (samdrift) |
| Sjöbo | `/sjobo` | Indecta OnlineKalender |
| Gislaved | `/gislaved` | Indecta OnlineKalender via SÅM (samdrift) |
| Gnosjö | `/gnosjo` | Indecta OnlineKalender via SÅM (samdrift) |
| Hylte | `/hylte` | Indecta OnlineKalender via SÅM (samdrift) |
| Vaggeryd | `/vaggeryd` | Indecta OnlineKalender via SÅM (samdrift) |
| Värnamo | `/varnamo` | Indecta OnlineKalender via SÅM (samdrift) |
| Skellefteå | `/skelleftea` | EDP Future / SimpleWastePickup |
| Boden | `/boden` | EDP Future / SimpleWastePickup |
| Uppsala | `/uppsala` | EDP Future via Uppsala Vatten |
| Borås | `/boras` | EDP Future via Borås Energi och Miljö |
| Mark | `/mark` | EDP Future via Marks kommun |
| Lycksele | `/lycksele` | EDP Future |
| Kiruna | `/kiruna` | EDP Future via Tekniska Verken |
| Lidköping | `/lidkoping` | EDP Future |
| Stenungsund | `/stenungsund` | EDP Future |
| Orust | `/orust` | EDP Future |
| Ljungby | `/ljungby` | EDP Future |
| Örebro | `/orebro` | EDP Future |
| Nacka | `/nacka` | EDP Future via NVOA |
| Ale | `/ale` | EDP Future |
| Kungälv | `/kungalv` | EDP Future |
| Herrljunga | `/herrljunga` | EDP Future via Remondis (samdrift) |
| Vårgårda | `/vargarda` | EDP Future via Remondis (samdrift) |
| Växjö | `/vaxjo` | EDP Future via SSAM (samdrift) |
| Älmhult | `/almhult` | EDP Future via SSAM (samdrift) |
| Tingsryd | `/tingsryd` | EDP Future via SSAM (samdrift) |
| Markaryd | `/markaryd` | EDP Future via SSAM (samdrift) |
| Lessebo | `/lessebo` | EDP Future via SSAM (samdrift) |
| Västerås | `/vasteras` | EDP Future via Vafab Miljö (samdrift) |
| Enköping | `/enkoping` | EDP Future via Vafab Miljö (samdrift) |
| Hallstahammar | `/hallstahammar` | EDP Future via Vafab Miljö (samdrift) |
| Heby | `/heby` | EDP Future via Vafab Miljö (samdrift) |
| Köping | `/koping` | EDP Future via Vafab Miljö (samdrift) |
| Norberg | `/norberg` | EDP Future via Vafab Miljö (samdrift) |
| Sala | `/sala` | EDP Future via Vafab Miljö (samdrift) |
| Skinnskatteberg | `/skinnskatteberg` | EDP Future via Vafab Miljö (samdrift) |
| Surahammar | `/surahammar` | EDP Future via Vafab Miljö (samdrift) |
| Fagersta | `/fagersta` | EDP Future via Vafab Miljö (samdrift) |
| Kungsör | `/kungsor` | EDP Future via Vafab Miljö (samdrift) |
| Arboga | `/arboga` | EDP Future via Vafab Miljö (samdrift) |
| Kalmar | `/kalmar` | EDP Future via Kretslopp Sydost (samdrift) |
| Mörbylånga | `/morbylanga` | EDP Future via Kretslopp Sydost (samdrift) |
| Nybro | `/nybro` | EDP Future via Kretslopp Sydost (samdrift) |
| Oskarshamn | `/oskarshamn` | EDP Future via Kretslopp Sydost (samdrift) |
| Torsås | `/torsas` | EDP Future via Kretslopp Sydost (samdrift) |
| Borgholm | `/borgholm` | EDP Future via Kretslopp Sydost (samdrift) |
| Mönsterås | `/monsteras` | EDP Future via Kretslopp Sydost (samdrift) |
| Hultsfred | `/hultsfred` | EDP Future via Kretslopp Sydost (samdrift) |
| Högsby | `/hogsby` | EDP Future via Kretslopp Sydost (samdrift) |
| Vetlanda | `/vetlanda` | EDP Future via Kretslopp Sydost (samdrift) |
| Sävsjö | `/savsjo` | EDP Future via Kretslopp Sydost (samdrift) |
| Uppvidinge | `/uppvidinge` | EDP Future via Kretslopp Sydost (samdrift) |
| Lindesberg | `/lindesberg` | EDP Future via Samhällsbyggnad Bergslagen (samdrift) |
| Nora | `/nora` | EDP Future via Samhällsbyggnad Bergslagen (samdrift) |
| Hällefors | `/hallefors` | EDP Future via Samhällsbyggnad Bergslagen (samdrift) |
| Ljusnarsberg | `/ljusnarsberg` | EDP Future via Samhällsbyggnad Bergslagen (samdrift) |
| Falkenberg | `/falkenberg` | EDP Future via Vivab (FutureWebFalken) |
| Jönköping | `/jonkoping` | EDP Future via June Avfall & Miljö (samdrift) |
| Habo | `/habo` | EDP Future via June Avfall & Miljö (samdrift) |
| Mullsjö | `/mullsjo` | EDP Future via June Avfall & Miljö (samdrift) |
| Ludvika | `/ludvika` | EDP Future via WBAB |
| Smedjebacken | `/smedjebacken` | EDP Future via WBAB |
| Kramfors | `/kramfors` | EDP Future via Kramfors kommun |
| Lidingö | `/lidingo` | EDP Future via Lidingö Vatten & Avfall |
| Lund | `/lund` | EDP Future via Lunds Renhållningsverk |
| Svenljunga | `/svenljunga` | EDP Future via Svenljunga kommun |
| Ekerö | `/ekero` | Roslagsvatten (Drupal-widget) |
| Vaxholm | `/vaxholm` | Roslagsvatten (Drupal-widget) |
| Österåker | `/osteraker` | Roslagsvatten (Drupal-widget) |
| Danderyd | `/danderyd` | EXDE Systems Mina sidor |
| Täby | `/taby` | EXDE Systems Mina sidor |
| Simrishamn | `/simrishamn` | EXDE Systems via Ökrab (samdrift) |
| Tomelilla | `/tomelilla` | EXDE Systems via Ökrab (samdrift) |
| Hässleholm | `/hassleholm` | Hässleholm Miljö (Appbolaget-API + SiteVision-webapp) |
| Härnösand | `/harnosand` | HEMAB (SiteVision-sökportlet, bulk-fetch via ?query=*) |
| Kristianstad | `/kristianstad` | Renhållningen Kristianstad (Appbolaget-universal API) |
| Katrineholm | `/katrineholm` | Sörmland Vatten & Avfall (WP admin-ajax) |
| Vingåker | `/vingaker` | Sörmland Vatten & Avfall (samdrift) |
| Flen | `/flen` | Sörmland Vatten & Avfall (samdrift) |
| Sundsvall | `/sundsvall` | **Officiell öppen data (CC0)** via dataportal.se |
| Botkyrka | `/sodertorn` | SRV Återvinning (Södertörn — samdrift) |
| Haninge | `/sodertorn` | SRV Återvinning (Södertörn — samdrift) |
| Huddinge | `/sodertorn` | SRV Återvinning (Södertörn — samdrift) |
| Nynäshamn | `/sodertorn` | SRV Återvinning (Södertörn — samdrift) |
| Salem | `/sodertorn` | SRV Återvinning (Södertörn — samdrift) |
| Essunga | `/essunga` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Falköping | `/falkoping` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Grästorp | `/grastorp` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Gullspång | `/gullspang` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Götene | `/gotene` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Hjo | `/hjo` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Karlsborg | `/karlsborg` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Mariestad | `/mariestad` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Skara | `/skara` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Skövde | `/skovde` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Tibro | `/tibro` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Töreboda | `/toreboda` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Vara | `/vara` | Avfallsappen via Avfall & Återvinning Skaraborg (samdrift) |
| Söderköping | `/soderkoping` | Avfallsappen (mobil-API) — opt-in via `SOPOR_AVFALLSAPPEN_MOBILE=1` |
| Motala | `/motala` | Avfallsappen (mobil-API) — opt-in via `SOPOR_AVFALLSAPPEN_MOBILE=1` |
| Vadstena | `/vadstena` | Avfallsappen (samdrift med Motala) — opt-in via `SOPOR_AVFALLSAPPEN_MOBILE=1` |
| Vallentuna | `/vallentuna` | Avfallsappen (mobil-API) — opt-in via `SOPOR_AVFALLSAPPEN_MOBILE=1` |
| Arjeplog | `/arjeplog` | Statiska rutt-slingor från arjeplog.se (ingen adress-uppslag) |
| Arvidsjaur | `/arvidsjaur` | Statiska rutt-slingor från arvidsjaur.se (ingen adress-uppslag) |
| Lysekil | `/lysekil` | Rambo AB (WP-JSON på rambo.se) |
| Munkedal | `/munkedal` | Rambo AB (samdrift) |
| Sotenäs | `/sotenas` | Rambo AB (samdrift) |
| Tanum | `/tanum` | Rambo AB (samdrift) |
| Kävlinge | `/kavlinge` | Sysav (publik Azure EDP-proxy) |
| Lomma | `/lomma` | Sysav (publik Azure EDP-proxy) |
| Svedala | `/svedala` | Sysav (publik Azure EDP-proxy) |
| Landskrona | `/landskrona` | LSR (Landskrona-Svalöv Renhållnings AB, öppet REST) |
| Svalöv | `/svalov` | LSR (samdrift) |
| Härjedalen | `/harjedalen` | Vatten och miljöresurs (SiteVision-webbapp) |
| Berg | `/berg` | Vatten och miljöresurs (samdrift) |
| Bräcke | `/bracke` | Vatten och miljöresurs (samdrift) |
| Forshaga | `/forshaga` | Optigon Avfallskollen (öppet REST) |
| Grums | `/grums` | Optigon Avfallskollen (samdrift) |
| Hammarö | `/hammaro` | Optigon Avfallskollen (samdrift) |
| Strömstad | `/stromstad` | Strömstads kommun (SiteVision-widget) |
| Bromölla | `/bromolla` | VMAB (jQuery + fullcalendar) |
| Ronneby | `/ronneby` | Ronneby Miljöteknik (samma software som VMAB) |
| Ydre | `/ydre` | Statisk rutt-slinga (Ydre Åkeri AB) |
| Sorsele | `/sorsele` | Statisk rutt-slinga från sorsele.se |
| Norsjö | `/norsjo` | Statisk rutt-slinga från norsjo.se |
| Bjurholm | `/bjurholm` | Statisk rutt-slinga från bjurholm.se |
| Malå | `/mala` | Statisk rutt-slinga från mala.se |
| Åre | `/are` | Statisk rutt-slinga (Lundstams Återvinning) |
| Åsele | `/asele` | Statisk rutt-slinga (SAV — Södra Lapplands Avfall) |
| Dorotea | `/dorotea` | Statisk rutt-slinga (SAV, samdrift med Åsele) |
| Jokkmokk | `/jokkmokk` | Statisk rutt-slinga (Jokkmokks Lastbilscentral) |
| Årjäng | `/arjang` | Statisk rutt-slinga från arjang.se |
| Valdemarsvik | `/valdemarsvik` | Statisk rutt-slinga från valdemarsvik.se |
| Vilhelmina | `/vilhelmina` | Statisk rutt-slinga från vilhelmina.se |
| Övertorneå | `/overtornea` | Statisk rutt-slinga (kompost varannan vecka) |
| Filipstad | `/filipstad` | Statisk rutt-slinga från filipstad.se |
| Storuman | `/storuman` | Statisk rutt-slinga från storuman.se |
| Vännäs | `/vannas` | Statisk rutt-slinga från vannas.se |
| Strömsund | `/stromsund` | Statisk rutt-slinga från stromsund.se |
| Ragunda | `/ragunda` | Statisk rutt-slinga (gäller t.o.m. 2026-12-31) |
| Pajala | `/pajala` | Statisk rutt-slinga (endast A-områden) |
| Alvesta | `/alvesta` | Alvesta Renhållnings AB — publik bulk-JSON på arabschema.alvesta.se |

## Endpoints

| Path | Beskrivning |
| --- | --- |
| `GET /` | Lista över stödda kommuner |
| `GET /:kommun` | HTML-formulär för en kommun |
| `GET /:kommun/autocomplete?query=<text>` | Adressförslag (JSON) |
| `GET /:kommun/preview?address=<full address>` | Hämtningstider (JSON) |
| `GET /:kommun/ics?address=<full address>` | iCalendar-feed (`text/calendar`) |
| `GET /healthz` | Health check |

## Lägga till en kommun

Beroende på vilken plattform kommunens avfallsbolag använder:

1. **SiteVision FetchPlanner (Limepark-widget)** — den enklaste vägen.
   Identifieras av `class="sv-limepark-app-fetchplanner"` eller
   `sv-limepark-webapp-fetchplanner` på sidan med adresssökning. Lägg
   till en post i `Registry::build` i `src/providers/mod.rs`:
   ```rust
   Arc::new(SitevisionFetchplanner::new(http.clone(), Config {
       id: "kommun-slug",
       name: "Kommunnamn",
       url: "https://exempel.se/sida-med-soksformularet",
       portlet_id: "12.xxxxxxxxxxxxxxxx",   // första arg till registerInitialState
       placeholder: "t.ex. Storgatan 1",
       note: "...",
       default_city: "Kommunnamn",
   })),
   ```
2. **Annan plattform** — skapa en ny fil under `src/providers/`, implementera
   `Provider`-traiten, och registrera den i `Registry::build`. Se
   `src/providers/stockholm.rs` för en JSON-API-implementation och
   `src/providers/sitevision_fetchplanner.rs` för en HTML-scraping-impl.

## Kartlagda plattformar (ej implementerade)

Identifierade men inte byggda än. Bidrag välkomna.

| Plattform | Markör | Sannolika kommuner |
| --- | --- | --- |
| **EDP Future / FutureWeb** (VertiGIS) | `futureweb.<kommun>.se`, `/EDPLogin/LogIn`, iframe-embed | Den i särklass största — uppskattningsvis 100+ kommuner. Bekräftade exempel: Luleå, Värmdö, Motala, Skellefteå, Hylte, Danderyd, Uppsala. Regionala bolag: NSR (NV Skåne: Helsingborg, Bjuv, Båstad, Höganäs, Åstorp, Ängelholm), Roslagsvatten (Österåker, Vaxholm, Knivsta, Ekerö, Vallentuna), Sörmland Vatten (Katrineholm, Flen, Vingåker), Vakin (Umeå, Nordmaling, Vindeln). |
| **Avfallsappen** (Bozzanova) | Kommunens sida länkar till app, mobil-API | ~50 kommuner. Lista på `avfallsappen.se`. |
| **Renova / Göteborg Stad Kretslopp och vatten** | Mina sidor med BankID | Göteborg (publika widget saknas — bara inloggad vy). |
| **Sysav-relaterade** | Per-kommun "Min sophämtning"-sidor | Lomma, Kävlinge, Svedala — sannolikt EDP-bakgrund. |
| **CGI BFUS** (Business For Utilities Suite) | `Portal-Version: CGI.Utility.Application.CPU.Client.Web.dll`, `pfu_lang`-cookie, `/Environments/<KLIENT>/`-paths | BankID/inloggning krävs — inga publika endpoints. Bekräftade kunder: Vakin (Umeå/Vindeln/Nordmaling), Stockholm Vatten och Avfall, Karlstads Energi, Alingsås Energi. CGI:s [produktsida](https://www.cgi.com/se/sv/business-for-utilities-suite) anger 70+ kunder utan publik lista. |

## Datakällor: gränsdragning

Coverage-tabellen ovan spänner över tre kvalitativt olika typer av
datakällor, och vi drar en explicit gräns för när vi använder dem:

1. **Officiell öppen data.** Sundsvall (CC0 via
   `api.sundsvall.se/Garbage/…`) är hittills enda kända fallet i
   Sverige. Om fler kommuner följer efter blir det här den rimliga
   vägen. Aktivt utan förbehåll.

2. **Publika kommun-widgets utan formell auth.** Merparten av
   provider-mappen — SiteVision-portlets, EDP FutureWeb, Indecta,
   VA SYD, Roslagsvatten, Optigon, m.fl. Ingen inloggning krävs,
   sidorna är byggda för att låta vem som helst slå upp en
   hämtningsdag, och scraping-ansträngningen är rimlig och
   proportionerlig mot nyttan. Aktivt utan förbehåll.

3. **App-leverantörens mobil-API utan Bearer/token.**
   Bozzanova/Avfallsappens WP-plugin-endpoints
   (`<tenant>.avfallsapp.se/wp-json/nova/v1/`) svarar helt utan
   autentiseringstoken; enda kravet är en själv-genererad UUID via
   `/register`. Rent tekniskt är alltså endpointen lika öppen som
   (2). **Men vi aktiverar den inte som default.** Designen
   signalerar att åtkomst är tänkt att gå via kommunens eller
   leverantörens app (plant_numbers nonce:as per requesting UUID på
   vissa tenanter, och Bozzanovas allmänna villkor täcker rimligen
   tredjepart-API-anrop). Vi avvaktar tills en kommun eller
   Bozzanova aktivt godkänner integrationen.

Gränsen mellan (2) och (3) är erkänt lite ologisk — den tekniska
nivån är samma, det är den *avsedda användningen* som skiljer. Vår
tumregel: om en publik sida på kommunens egen domän låter vem som
helst slå upp en adress bygger vi. Om åtkomsten är tänkt att gå via
en app-leverantörs klient och de har signalerat detta i designen
väntar vi på grönt ljus, även om endpointen råkar svara ändå.

### Kommuner sannolikt möjliga via app-API (bakom feature toggle)

Endpoint-signaturerna är verifierade att fungera live men rutterna
registreras inte utan `SOPOR_AVFALLSAPPEN_MOBILE=1` (se
[Miljövariabler](#miljövariabler)). Markerade `🚧` i 290-grid nedan.

| Kommun | Bozzanova-tenant | Notering |
| --- | --- | --- |
| Söderköping | `soderkoping.avfallsapp.se` | Permanent numeriskt plant_id |
| Motala | `motala.avfallsapp.se` | Samdrift med Vadstena |
| Vadstena | `motala.avfallsapp.se` | Adressuppslag via Motala-tenanten |
| Vallentuna | `vallentuna.avfallsapp.se` | Plant_number nonce:as per UUID |

Utöver dessa har vi kartlagt fler Bozzanova-tenanter via certificate
transparency-loggar (Sysav, Vafab Miljö, Rambo, Miva, June Avfall &
Miljö m.fl.). Flera av dem täcker kommuner som redan har öppna vägar
via andra providers och behöver därför inte den här fallbacken.

Testat men uteslutet: Dala Vatten och Avfall (`dalavatten` — Gagnef,
Leksand, Rättvik, Vansbro). Server-side-blockering av `/list` för
nya device-UUIDs oavsett flöde. Endpointerna finns dokumenterade i
`providers/avfallsappen.rs` — återöppna om beteendet förändras.

Ⓑ-markeringen utelämnar också ett par tenanter medvetet: `teknikivast`
(Arvika, Eda) och `vanersborg` (Vänersborg) svarar båda på `/api/nova/v1/`
med en statisk Bearer + `X-App-Identifier` (samma widget-mode-mönster
som AÅS Skaraborg), men — till skillnad från AÅS — hostas ingen widget
på kommunernas publika webb. Bearer:n finns bara inuti mobilklienten
(deras `Mitt DVA`-motsvarigheter, båda NativeScript-appar med
plaintext-bundle). Att extrahera nycklar ur en mobil-APK ligger en nivå
mer intrikat än att läsa en publik web-widget, så vi markerar dem inte
som "potentiellt integrerbar" här. Samma resonemang för övriga
`requires_token: True`-tenanter i [HACS-referensen](https://github.com/mampfes/hacs_waste_collection_schedule).

## Alla Sveriges 290 kommuner

`✅` = stöds nu · `🚧` = möjlig via app-leverantörens API men opt-in
bakom feature toggle · `🔬` = plattform identifierad, ej implementerad
· `⬜` = ej undersökt.

Sekundär markering `Ⓑ` = Bozzanova/Avfallsappen-tenant existerar för
kommunen (verifierat via CT-loggar eller känd bolagstillhörighet) och
den är därmed *potentiellt* integrerbar via mobil-API-vägen (se
[Datakällor: gränsdragning](#datakällor-gränsdragning) ovan). Överlapp
med primär markering är väntat: vissa `✅` täcks redan via en annan
och mer öppen väg (Bozzanova-fallbacken behövs inte), vissa `🚧` är
just den här mobil-mode-gate:n, och vissa `⬜` har en tenant men är
inte utvärderade.

| | | | | |
| --- | --- | --- | --- | --- |
| ✅ Ale | ⬜ Alingsås | ✅ Alvesta | ⬜ Aneby | ✅ Arboga |
| ✅ Arjeplog | ✅ Arvidsjaur | ⬜ Arvika | ⬜ Askersund | ⬜ Avesta |
| ⬜ Bengtsfors | ✅ Berg | ✅ Bjurholm | 🔬 Bjuv | ✅ Boden |
| ⬜ Bollebygd | ⬜ Bollnäs | ✅ Borgholm | ⬜ Borlänge | ✅ Borås Ⓑ |
| ✅ Botkyrka | ⬜ Boxholm Ⓑ | ✅ Bromölla | ✅ Bräcke | ✅ Burlöv |
| 🔬 Båstad | ⬜ Dals-Ed | ✅ Danderyd | ⬜ Degerfors | ✅ Dorotea |
| ⬜ Eda | ✅ Ekerö | ⬜ Eksjö | ⬜ Emmaboda | ✅ Enköping |
| ⬜ Eskilstuna | ⬜ Eslöv | ✅ Essunga Ⓑ | ✅ Fagersta | ✅ Falkenberg |
| ✅ Falköping Ⓑ | ✅ Falun | ✅ Filipstad | ⬜ Finspång Ⓑ | ✅ Flen |
| ✅ Forshaga Ⓑ | ⬜ Färgelanda | ⬜ Gagnef Ⓑ | ✅ Gislaved | ⬜ Gnesta |
| ✅ Gnosjö | ⬜ Gotland | ✅ Grums Ⓑ | ✅ Grästorp Ⓑ | ✅ Gullspång Ⓑ |
| ⬜ Gällivare | ⬜ Gävle | ⬜ Göteborg | ✅ Götene Ⓑ | ✅ Habo Ⓑ |
| ⬜ Hagfors Ⓑ | ⬜ Hallsberg | ✅ Hallstahammar | ⬜ Halmstad | ✅ Hammarö Ⓑ |
| ✅ Haninge | ⬜ Haparanda | ✅ Heby | ⬜ Hedemora | 🔬 Helsingborg |
| ✅ Herrljunga | ✅ Hjo Ⓑ | ⬜ Hofors | ✅ Huddinge | ⬜ Hudiksvall Ⓑ |
| ✅ Hultsfred | ✅ Hylte | ⬜ Håbo | ✅ Hällefors | ✅ Härjedalen |
| ✅ Härnösand | ⬜ Härryda | ✅ Hässleholm | 🔬 Höganäs | ✅ Högsby |
| ⬜ Hörby | ⬜ Höör | ✅ Jokkmokk | ⬜ Järfälla | ✅ Jönköping Ⓑ |
| ⬜ Kalix Ⓑ | ✅ Kalmar | ✅ Karlsborg Ⓑ | ⬜ Karlshamn | ⬜ Karlskoga |
| ⬜ Karlskrona | ⬜ Karlstad | ✅ Katrineholm | ⬜ Kil Ⓑ | ⬜ Kinda Ⓑ |
| ✅ Kiruna | ⬜ Klippan | 🔬 Knivsta Ⓑ | ✅ Kramfors | ✅ Kristianstad |
| ⬜ Kristinehamn | ⬜ Krokom Ⓑ | ⬜ Kumla Ⓑ | ⬜ Kungsbacka Ⓑ | ✅ Kungsör |
| ✅ Kungälv Ⓑ | ✅ Kävlinge | ✅ Köping | ⬜ Laholm | ✅ Landskrona |
| ⬜ Laxå | ⬜ Lekeberg | ⬜ Leksand Ⓑ | ⬜ Lerum Ⓑ | ✅ Lessebo |
| ✅ Lidingö | ✅ Lidköping | ⬜ Lilla Edet | ✅ Lindesberg | ⬜ Linköping |
| ✅ Ljungby | ⬜ Ljusdal | ✅ Ljusnarsberg | ✅ Lomma | ✅ Ludvika |
| 🔬 Luleå | ✅ Lund | ✅ Lycksele | ✅ Lysekil Ⓑ | ✅ Malmö |
| ⬜ Malung-Sälen | ✅ Malå | ✅ Mariestad Ⓑ | ✅ Mark | ✅ Markaryd |
| ⬜ Mellerud | ⬜ Mjölby | ⬜ Mora Ⓑ | 🚧 Motala Ⓑ | ✅ Mullsjö Ⓑ |
| ✅ Munkedal Ⓑ | ⬜ Munkfors Ⓑ | ⬜ Mölndal Ⓑ | ✅ Mönsterås | ✅ Mörbylånga |
| ✅ Nacka Ⓑ | ✅ Nora | ✅ Norberg | ⬜ Nordanstig | 🔬 Nordmaling Ⓑ |
| ⬜ Norrköping | ⬜ Norrtälje | ✅ Norsjö | ✅ Nybro | ⬜ Nykvarn Ⓑ |
| ⬜ Nyköping | ✅ Nynäshamn | ⬜ Nässjö | ⬜ Ockelbo | ⬜ Olofström |
| ⬜ Orsa Ⓑ | ✅ Orust | ✅ Osby | ✅ Oskarshamn | ⬜ Ovanåker |
| ⬜ Oxelösund | ✅ Pajala | ⬜ Partille | ⬜ Perstorp | ⬜ Piteå |
| ✅ Ragunda Ⓑ | ⬜ Robertsfors | ✅ Ronneby | ⬜ Rättvik Ⓑ | ✅ Sala |
| ✅ Salem | ⬜ Sandviken | ⬜ Sigtuna Ⓑ | ✅ Simrishamn | ✅ Sjöbo |
| ✅ Skara Ⓑ | ✅ Skellefteå | ✅ Skinnskatteberg | ⬜ Skurup | ✅ Skövde Ⓑ |
| ✅ Smedjebacken | ⬜ Sollefteå Ⓑ | ⬜ Sollentuna | ⬜ Solna | ✅ Sorsele |
| ✅ Sotenäs Ⓑ | ⬜ Staffanstorp | ✅ Stenungsund | ✅ Stockholm | ⬜ Storfors |
| ✅ Storuman | ⬜ Strängnäs | ✅ Strömstad | ✅ Strömsund Ⓑ | ⬜ Sundbyberg |
| ✅ Sundsvall | ⬜ Sunne Ⓑ | ✅ Surahammar | ✅ Svalöv | ✅ Svedala |
| ✅ Svenljunga | ⬜ Säffle | ⬜ Säter | ✅ Sävsjö | ⬜ Söderhamn Ⓑ |
| 🚧 Söderköping Ⓑ | ⬜ Södertälje Ⓑ | ⬜ Sölvesborg | ✅ Tanum Ⓑ | ✅ Tibro Ⓑ |
| ⬜ Tidaholm | ⬜ Tierp | ⬜ Timrå | ✅ Tingsryd | ⬜ Tjörn |
| ✅ Tomelilla | ⬜ Torsby Ⓑ | ✅ Torsås | ⬜ Tranemo Ⓑ | ⬜ Tranås |
| ⬜ Trelleborg | ⬜ Trollhättan | ⬜ Trosa | ⬜ Tyresö Ⓑ | ✅ Täby |
| ✅ Töreboda Ⓑ | ⬜ Uddevalla | ⬜ Ulricehamn Ⓑ | 🔬 Umeå | ⬜ Upplands-Bro Ⓑ |
| ⬜ Upplands Väsby | ✅ Uppsala | ✅ Uppvidinge | 🚧 Vadstena Ⓑ | ✅ Vaggeryd Ⓑ |
| ✅ Valdemarsvik Ⓑ | 🚧 Vallentuna Ⓑ | ⬜ Vansbro Ⓑ | ✅ Vara Ⓑ | ⬜ Varberg |
| ✅ Vaxholm | ⬜ Vellinge | ✅ Vetlanda | ✅ Vilhelmina | ⬜ Vimmerby |
| 🔬 Vindeln Ⓑ | ✅ Vingåker | ✅ Vårgårda | ⬜ Vänersborg | ✅ Vännäs |
| 🔬 Värmdö | ✅ Värnamo | ⬜ Västervik | ✅ Västerås | ✅ Växjö |
| ✅ Ydre | ⬜ Ystad | ⬜ Åmål | ⬜ Ånge | ✅ Åre |
| ✅ Årjäng | ✅ Åsele | 🔬 Åstorp | ⬜ Åtvidaberg Ⓑ | ✅ Älmhult |
| ⬜ Älvdalen Ⓑ | ⬜ Älvkarleby | ⬜ Älvsbyn | 🔬 Ängelholm | ⬜ Öckerö |
| ⬜ Ödeshög Ⓑ | ✅ Örebro | ⬜ Örkelljunga | ✅ Örnsköldsvik Ⓑ | ⬜ Östersund Ⓑ |
| ✅ Österåker | ⬜ Östhammar | ✅ Östra Göinge | ⬜ Överkalix | ✅ Övertorneå |

Status motsvarar status i kodbasen idag. 🔬 betyder att jag identifierat
sannolik plattform via offentlig källa men inte verifierat eller byggt
adapter. ⬜ kan vara EDP, Avfallsappen, eller en lokal lösning — behöver
undersökas per kommun.

## Kör lokalt

```sh
cargo run
# open http://localhost:8080
```

## Docker

```sh
docker build -t sopor .
docker run --rm -p 8080:8080 sopor
```

## Miljövariabler

- `PORT` (default `8080`)
- `RUST_LOG` (default `info`)
- `SOPOR_AVFALLSAPPEN_MOBILE` (default `av`) — sätts till `1`, `true`,
  `yes` eller `on` för att aktivera Avfallsappen-mobile-API-fallbacken
  (Söderköping, Motala, Vadstena, Vallentuna). Läget använder ingen
  Bearer-token utan bara en själv-genererad UUID via `/register`;
  det är fortfarande en tredjepart-integration som lämpligen bör
  koordineras med Bozzanova innan produktion (se moduldokumentation i
  `src/providers/avfallsappen.rs`). Widget-mode för AÅS-Skaraborg
  (Falköping, Skövde m.fl.) är inte gated och aktivt oavsett flaggan.
  Mobile-app-parsers har egna unit-tester som är `#[ignore]:ade` som
  default — kör dem via `cargo test -- --include-ignored`.

## Notiser

- Feeden innehåller `VALARM` 6 timmar före midnatt på hämtningsdagen, vilket
  ger notis ~18:00 dagen innan.
- Apple Calendar respekterar `VALARM` från prenumerationer ✓
- Google Calendar på Android ignorerar `VALARM` från prenumerationer.
  Använd t.ex. [ICSx⁵](https://icsx5.bitfire.at/) för lokala Android-notiser.

## Licens

GPL-3.0-only. Se [`LICENSE`](LICENSE).

## Anmärkningar

Inofficiell tjänst — kontakta din kommun för officiella uppgifter.
