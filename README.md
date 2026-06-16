# sopor

Kalenderprenumeration (iCalendar) för sophämtning i Stockholms stad.

Frontend (HTML/JS) söker upp en adress med autocomplete, visar nästa hämtningar
och genererar en `webcal://`/`https://` prenumerationslänk som kan läggas till
i Google Calendar, Apple Calendar eller Outlook.

Backend (Rust + Axum) hämtar data live från Stockholm Vatten och Avfall
(`stockholmvattenochavfall.se`) och returnerar en `text/calendar`-feed med
återkommande events (RRULE) baserat på upphämtningsfrekvensen.

## Endpoints

| Path | Beskrivning |
| --- | --- |
| `GET /` | HTML-formulär |
| `GET /autocomplete?query=<text>` | Proxar SVOAs adressförslag |
| `GET /preview?address=<full address>` | JSON med hämtningstider |
| `GET /ics?address=<full address>` | iCalendar-feed (`text/calendar`) |
| `GET /healthz` | Health check |

`address` ska vara hela strängen från autocomplete-värdet, t.ex.
`Olovslundsvägen 9, Bromma, 167 72`.

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

Eller med compose: `docker compose up --build`.

## Miljövariabler

- `PORT` (default `8080`)
- `RUST_LOG` (default `info`)

## Anmärkningar

- Endast villor/radhus i Stockholms stad (SVOA:s täckning).
  Flerfamiljshus och samfälligheter får tomma resultat.
- Återkommande events projiceras ~1 år framåt med RRULE; klienten
  hämtar feeden periodiskt och uppdaterar serien.
- Inofficiell tjänst — ingen affiliering med Stockholm Vatten och Avfall.

## Licens

GPL-3.0-only. Se [`LICENSE`](LICENSE).
