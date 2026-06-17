# sopor

Kalenderprenumeration (iCalendar) för sophämtning i svenska kommuner.

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

## Alla Sveriges 290 kommuner

`✅` = stöds nu · `🔬` = plattform identifierad, ej implementerad · `⬜` = ej undersökt.

| | | | | |
| --- | --- | --- | --- | --- |
| ⬜ Ale | ⬜ Alingsås | ⬜ Alvesta | ⬜ Aneby | ⬜ Arboga |
| ⬜ Arjeplog | ⬜ Arvidsjaur | ⬜ Arvika | ⬜ Askersund | ⬜ Avesta |
| ⬜ Bengtsfors | ⬜ Berg | ⬜ Bjurholm | 🔬 Bjuv | ⬜ Boden |
| ⬜ Bollebygd | ⬜ Bollnäs | ⬜ Borgholm | ⬜ Borlänge | ⬜ Borås |
| ⬜ Botkyrka | ⬜ Boxholm | ⬜ Bromölla | ⬜ Bräcke | ✅ Burlöv |
| 🔬 Båstad | ⬜ Dals-Ed | 🔬 Danderyd | ⬜ Degerfors | ⬜ Dorotea |
| ⬜ Eda | 🔬 Ekerö | ⬜ Eksjö | ⬜ Emmaboda | ⬜ Enköping |
| ⬜ Eskilstuna | ⬜ Eslöv | ⬜ Essunga | ⬜ Fagersta | ⬜ Falkenberg |
| ⬜ Falköping | ✅ Falun | ⬜ Filipstad | ⬜ Finspång | 🔬 Flen |
| ⬜ Forshaga | ⬜ Färgelanda | ⬜ Gagnef | ⬜ Gislaved | ⬜ Gnesta |
| ⬜ Gnosjö | ⬜ Gotland | ⬜ Grums | ⬜ Grästorp | ⬜ Gullspång |
| ⬜ Gällivare | ⬜ Gävle | ⬜ Göteborg | ⬜ Götene | ⬜ Habo |
| ⬜ Hagfors | ⬜ Hallsberg | ⬜ Hallstahammar | ⬜ Halmstad | ⬜ Hammarö |
| ⬜ Haninge | ⬜ Haparanda | ⬜ Heby | ⬜ Hedemora | 🔬 Helsingborg |
| ⬜ Herrljunga | ⬜ Hjo | ⬜ Hofors | ⬜ Huddinge | ⬜ Hudiksvall |
| ⬜ Hultsfred | 🔬 Hylte | ⬜ Håbo | ⬜ Hällefors | ⬜ Härjedalen |
| ⬜ Härnösand | ⬜ Härryda | ⬜ Hässleholm | 🔬 Höganäs | ⬜ Högsby |
| ⬜ Hörby | ⬜ Höör | ⬜ Jokkmokk | ⬜ Järfälla | ⬜ Jönköping |
| ⬜ Kalix | ⬜ Kalmar | ⬜ Karlsborg | ⬜ Karlshamn | ⬜ Karlskoga |
| ⬜ Karlskrona | ⬜ Karlstad | 🔬 Katrineholm | ⬜ Kil | ⬜ Kinda |
| ⬜ Kiruna | ⬜ Klippan | 🔬 Knivsta | ⬜ Kramfors | ⬜ Kristianstad |
| ⬜ Kristinehamn | ⬜ Krokom | ⬜ Kumla | ⬜ Kungsbacka | ⬜ Kungsör |
| ⬜ Kungälv | ⬜ Kävlinge | ⬜ Köping | ⬜ Laholm | ⬜ Landskrona |
| ⬜ Laxå | ⬜ Lekeberg | ⬜ Leksand | ⬜ Lerum | ⬜ Lessebo |
| ⬜ Lidingö | ⬜ Lidköping | ⬜ Lilla Edet | ⬜ Lindesberg | ⬜ Linköping |
| ⬜ Ljungby | ⬜ Ljusdal | ⬜ Ljusnarsberg | ⬜ Lomma | ⬜ Ludvika |
| 🔬 Luleå | ⬜ Lund | ⬜ Lycksele | ⬜ Lysekil | ✅ Malmö |
| ⬜ Malung-Sälen | ⬜ Malå | ⬜ Mariestad | ⬜ Mark | ⬜ Markaryd |
| ⬜ Mellerud | ⬜ Mjölby | ⬜ Mora | 🔬 Motala | ⬜ Mullsjö |
| ⬜ Munkedal | ⬜ Munkfors | ⬜ Mölndal | ⬜ Mönsterås | ⬜ Mörbylånga |
| ⬜ Nacka | ⬜ Nora | ⬜ Norberg | ⬜ Nordanstig | 🔬 Nordmaling |
| ⬜ Norrköping | ⬜ Norrtälje | ⬜ Norsjö | ⬜ Nybro | ⬜ Nykvarn |
| ⬜ Nyköping | ⬜ Nynäshamn | ⬜ Nässjö | ⬜ Ockelbo | ⬜ Olofström |
| ⬜ Orsa | ⬜ Orust | ⬜ Osby | ⬜ Oskarshamn | ⬜ Ovanåker |
| ⬜ Oxelösund | ⬜ Pajala | ⬜ Partille | ⬜ Perstorp | ⬜ Piteå |
| ⬜ Ragunda | ⬜ Robertsfors | ⬜ Ronneby | ⬜ Rättvik | ⬜ Sala |
| ⬜ Salem | ⬜ Sandviken | ⬜ Sigtuna | ⬜ Simrishamn | ⬜ Sjöbo |
| ⬜ Skara | 🔬 Skellefteå | ⬜ Skinnskatteberg | ⬜ Skurup | ⬜ Skövde |
| ⬜ Smedjebacken | ⬜ Sollefteå | ⬜ Sollentuna | ⬜ Solna | ⬜ Sorsele |
| ⬜ Sotenäs | ⬜ Staffanstorp | ⬜ Stenungsund | ✅ Stockholm | ⬜ Storfors |
| ⬜ Storuman | ⬜ Strängnäs | ⬜ Strömstad | ⬜ Strömsund | ⬜ Sundbyberg |
| ⬜ Sundsvall | ⬜ Sunne | ⬜ Surahammar | ⬜ Svalöv | ⬜ Svedala |
| ⬜ Svenljunga | ⬜ Säffle | ⬜ Säter | ⬜ Sävsjö | ⬜ Söderhamn |
| ⬜ Söderköping | ⬜ Södertälje | ⬜ Sölvesborg | ⬜ Tanum | ⬜ Tibro |
| ⬜ Tidaholm | ⬜ Tierp | ⬜ Timrå | ⬜ Tingsryd | ⬜ Tjörn |
| ⬜ Tomelilla | ⬜ Torsby | ⬜ Torsås | ⬜ Tranemo | ⬜ Tranås |
| ⬜ Trelleborg | ⬜ Trollhättan | ⬜ Trosa | ⬜ Tyresö | ⬜ Täby |
| ⬜ Töreboda | ⬜ Uddevalla | ⬜ Ulricehamn | 🔬 Umeå | ⬜ Upplands-Bro |
| ⬜ Upplands Väsby | 🔬 Uppsala | ⬜ Uppvidinge | ⬜ Vadstena | ⬜ Vaggeryd |
| ⬜ Valdemarsvik | 🔬 Vallentuna | ⬜ Vansbro | ⬜ Vara | ⬜ Varberg |
| 🔬 Vaxholm | ⬜ Vellinge | ⬜ Vetlanda | ⬜ Vilhelmina | ⬜ Vimmerby |
| 🔬 Vindeln | 🔬 Vingåker | ⬜ Vårgårda | ⬜ Vänersborg | ⬜ Vännäs |
| 🔬 Värmdö | ⬜ Värnamo | ⬜ Västervik | ⬜ Västerås | ⬜ Växjö |
| ⬜ Ydre | ⬜ Ystad | ⬜ Åmål | ⬜ Ånge | ⬜ Åre |
| ⬜ Årjäng | ⬜ Åsele | 🔬 Åstorp | ⬜ Åtvidaberg | ⬜ Älmhult |
| ⬜ Älvdalen | ⬜ Älvkarleby | ⬜ Älvsbyn | 🔬 Ängelholm | ⬜ Öckerö |
| ⬜ Ödeshög | ⬜ Örebro | ⬜ Örkelljunga | ✅ Örnsköldsvik | ⬜ Östersund |
| 🔬 Österåker | ⬜ Östhammar | ⬜ Östra Göinge | ⬜ Överkalix | ⬜ Övertorneå |

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
