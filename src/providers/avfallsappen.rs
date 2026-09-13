//! Multi-tenant provider för kommuner på Bozzanovas Avfallsappen-plattform.
//! Två distinkta lägen stöds:
//!
//! * [`Auth::Widget`] — Laravel-baserad widget-mode på
//!   `https://<tenant>.avfallsapp.se/api/nova/v1/next-pickup/`. Kräver en
//!   statisk Bearer + `X-App-Identifier` som extraheras ur en publik
//!   Vue-widget på kommunsajten (som skickar samma två strängar på varje
//!   anrop). Första tenanten: `gullspang` → 13 AÅS-Skaraborg-kommuner via
//!   widgeten inbäddad på `avfallskaraborg.se`.
//!
//! * [`Auth::MobileApp`] — WordPress-plugin-mode på
//!   `https://<tenant>.avfallsapp.se/wp-json/nova/v1/`. Har inga
//!   Bearer-krav; kräver bara en engångs-`/register` med en själv-genererad
//!   UUID som sen används som `X-App-Identifier`. Detta läge används
//!   **endast** för tenanter som inte är auth-skyddade — vi genererar en
//!   ny UUID per `schedule()`-anrop och kastar bort efteråt (ingen
//!   persistent state).
//!
//! ### Widget-mode flöde
//! ```text
//!   GET  /api/nova/v1/next-pickup/search?address=<street>
//!        → { "<Ort>": [{address, zip_city, plant_number, type}, ...], ... }
//!   POST /api/nova/v1/next-pickup/address?plant_id=<plant_number>
//!        → { address, city, bins: [{type, pickup_date}, ...] }
//! ```
//!
//! ### Mobile-app-mode flöde
//! ```text
//!   POST /wp-json/nova/v1/register              {identifier, uuid, platform, ...}
//!   GET  /wp-json/nova/v1/next-pickup/search-flat?address=<street>
//!        → [{key, is_key}, {plant_number, address, zip_city}, ...] (flat)
//!   POST /wp-json/nova/v1/next-pickup/set-status
//!        {plant_id, address_enabled:true, notification_enabled:true}
//!   GET  /wp-json/nova/v1/next-pickup/list
//!        → [{address, city, plant_id, bins: [{type, pickup_date, ...}]}]
//! ```
//!
//! ### `plant_number` semantik
//! Både widget-läget och mobile-app-läget returnerar ett `plant_number`
//! som är opakt för oss (Laravel-krypterad blob i widget-läget, WP-nonce-
//! eller stabilt id i mobile-app-läget). Vi exponerar det **aldrig** i
//! autocomplete-värden — istället presenteras `"<street>, <ort>"` för
//! användaren och vi söker om vid `schedule()` för att lösa upp adress →
//! plant_number igen. Detta gör att `plant_number`-rotationer inte
//! spelar någon roll för våra ical-URL:er.
//!
//! ### Data-cadence
//! Oavsett läge returnerar upstream bara **nästa** hämtning per fraktion.
//! Vi emitterar därför en enskild anchor-datum per `PickupSeries` utan
//! RRULE och förlitar oss på iCal-klientens auto-refresh (~12 h) för att
//! hämta nästa datum efter att den aktuella passerat.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

// Genererar en UUID-formaterad sträng ur SHA-256(process-räknare + tid).
// Servern accepterar godtycklig sträng som `X-App-Identifier`, men vi
// följer UUID-formen så att register-endpointens SQL-validering (som
// förväntar sig en `uuid`-typad kolumn) inte bråkar. Uniqueness krävs
// bara inom en process — vi registrerar en ny UUID per schedule()-anrop
// och kastar bort den efteråt.
static UUID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_uuid_v4_like() -> String {
    let counter = UUID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(nanos.to_le_bytes());
    hasher.update(counter.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    let digest = hasher.finalize();
    // Formatera de första 16 byten som UUID xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx.
    // Sätt version-nibble = 4 och variant-nibble = 8..b för att efterlikna
    // UUIDv4 (server kan reject på formatkontroll).
    let mut b = [0u8; 16];
    b.copy_from_slice(&digest[..16]);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15],
    )
}

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Subdomän under `avfallsapp.se`.
    pub tenant: &'static str,
    /// Autentiseringsläge — se moduldokumentationen.
    pub auth: Auth,
    /// Tillåt-lista av `zip_city`-värden som hör till den här kommunen.
    /// Matchas case-insensitivt (Unicode). Adresser från andra kommuner
    /// som råkar dela tenant (multi-kommun-bolag) filtreras bort.
    pub cities: &'static [&'static str],
}

#[derive(Clone)]
pub enum Auth {
    /// Statiska nycklar extraherade ur en publik JavaScript-widget på
    /// kommunsajten. Använd för tenanter där Bozzanova hostar en Laravel-
    /// baserad widget och Bearer-tokenen är hårdkodad i den utskickade
    /// JS-bundeln (dvs. reellt sett offentlig).
    Widget {
        bearer: &'static str,
        app_identifier: &'static str,
    },
    /// WordPress-plugin-läget utan Bearer. Använd **endast** för tenanter
    /// där `wp-json/nova/v1/`-endpointerna är öppna (skyddade endast av
    /// en själv-genererad UUID via `/register`).
    MobileApp,
}

pub struct Avfallsappen {
    http: reqwest::Client,
    cfg: Config,
}

impl Avfallsappen {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg
            .cities
            .iter()
            .any(|c| c.to_lowercase() == needle)
    }

    // --- Widget-läge ----------------------------------------------------
    fn widget_base(&self) -> String {
        format!(
            "https://{}.avfallsapp.se/api/nova/v1/next-pickup",
            self.cfg.tenant
        )
    }

    async fn widget_search(
        &self,
        query: &str,
        bearer: &str,
        app_id: &str,
    ) -> Result<Vec<SearchHit>, ProviderError> {
        let url = format!("{}/search", self.widget_base());
        let resp = self
            .http
            .get(&url)
            .query(&[("address", query)])
            .header("Authorization", format!("Bearer {bearer}"))
            .header("X-App-Identifier", app_id)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let text = self.check_auth(resp).await?.text().await?;
        parse_widget_search(&text)
    }

    async fn widget_fetch_pickup(
        &self,
        plant_number: &str,
        bearer: &str,
        app_id: &str,
    ) -> Result<PickupResp, ProviderError> {
        let url = format!("{}/address", self.widget_base());
        let resp = self
            .http
            .post(&url)
            .query(&[("plant_id", plant_number)])
            .header("Authorization", format!("Bearer {bearer}"))
            .header("X-App-Identifier", app_id)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .body("")
            .send()
            .await?;
        let text = self.check_auth(resp).await?.text().await?;
        Ok(serde_json::from_str(&text)?)
    }

    // --- Mobile-app-läge -----------------------------------------------
    fn wp_base(&self) -> String {
        format!("https://{}.avfallsapp.se/wp-json/nova/v1", self.cfg.tenant)
    }

    async fn wp_register(&self, uuid: &str) -> Result<(), ProviderError> {
        // Server-side kolumnerna `platform`, `version`, `os_version`,
        // `model` är NOT NULL — utan dem svarar upstream med 400 och
        // en läckt SQL-error. Vi skickar en fast Android-etikett; värdena
        // används bara för statistik och påverkar inte schema-svaret.
        let body = serde_json::json!({
            "identifier": uuid,
            "uuid": uuid,
            "platform": "android",
            "version": 10301,
            "os_version": "14",
            "model": "sopor-motrice-bridge",
            "test": false,
        });
        let url = format!("{}/register", self.wp_base());
        let resp = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-App-Identifier", uuid)
            .body(body.to_string())
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    async fn wp_search(&self, query: &str) -> Result<Vec<SearchHit>, ProviderError> {
        // Använd `/next-pickup/search` (map-format) i stället för
        // `/search-flat`: både DVA-, Söderköping-, Motala- och Vallentuna-
        // tenanterna svarar konsekvent här, och plant_numbers härifrån
        // valideras av set-status (search-flat gav ibland non-bindable
        // varianter på DVA). Vissa tenanter kräver `X-App-Identifier`
        // för sökningen — skicka alltid en genererad UUID.
        let uuid = generate_uuid_v4_like();
        let url = format!("{}/next-pickup/search", self.wp_base());
        let resp = self
            .http
            .get(&url)
            .query(&[("address", query)])
            .header("X-App-Identifier", &uuid)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let text = resp.error_for_status()?.text().await?;
        let raw = parse_widget_search(&text)?;
        // Strippa ev. "NNN NN "-postnummerprefix ur zip_city så att
        // cities allow-listen inte behöver innehålla postnummer.
        Ok(raw
            .into_iter()
            .map(|h| SearchHit {
                address: h.address,
                zip_city: strip_zip_prefix(&h.zip_city),
                plant_number: h.plant_number,
            })
            .collect())
    }

    async fn wp_search_with_uuid(
        &self,
        query: &str,
        uuid: &str,
    ) -> Result<Vec<SearchHit>, ProviderError> {
        // Söker med en specifik UUID i stället för en tillfällig. Behövs
        // för tenanter (t.ex. Vallentuna) där search-svarets `plant_number`
        // är UUID-scopad — plant måste hämtas med samma UUID som senare
        // används för set-status.
        let url = format!("{}/next-pickup/search", self.wp_base());
        let resp = self
            .http
            .get(&url)
            .query(&[("address", query)])
            .header("X-App-Identifier", uuid)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let text = resp.error_for_status()?.text().await?;
        let raw = parse_widget_search(&text)?;
        Ok(raw
            .into_iter()
            .map(|h| SearchHit {
                address: h.address,
                zip_city: strip_zip_prefix(&h.zip_city),
                plant_number: h.plant_number,
            })
            .collect())
    }

    async fn wp_schedule_atomic(
        &self,
        street: &str,
        city_hint: Option<&str>,
    ) -> Result<(String, Vec<PickupSeries>), ProviderError> {
        // Håller EN UUID genom hela flödet: register → search →
        // set-status → list. Vissa tenanter (Vallentuna, DVA) nonce:ar
        // plant_number per requesting UUID, så search + set-status måste
        // ske under samma identitet.
        let uuid = generate_uuid_v4_like();
        self.wp_register(&uuid).await?;

        let hits = self.wp_search_with_uuid(street, &uuid).await?;
        let matched = hits
            .iter()
            .find(|h| {
                self.matches_city(&h.zip_city)
                    && h.address.eq_ignore_ascii_case(street)
                    && city_hint
                        .map(|c| h.zip_city.to_lowercase() == c.to_lowercase())
                        .unwrap_or(true)
            })
            .or_else(|| {
                hits.iter().find(|h| {
                    self.matches_city(&h.zip_city) && h.address.eq_ignore_ascii_case(street)
                })
            })
            .or_else(|| hits.iter().find(|h| self.matches_city(&h.zip_city)));

        let Some(hit) = matched else {
            return Ok((String::new(), Vec::new()));
        };

        let set_status_url = format!("{}/next-pickup/set-status", self.wp_base());
        let body = serde_json::json!({
            "plant_id": hit.plant_number,
            "address_enabled": true,
            "notification_enabled": true,
        });
        let resp = self
            .http
            .post(&set_status_url)
            .header("Content-Type", "application/json")
            .header("X-App-Identifier", &uuid)
            .body(body.to_string())
            .send()
            .await?;
        resp.error_for_status()?;

        let list_url = format!("{}/next-pickup/list", self.wp_base());
        let list_resp = self
            .http
            .get(&list_url)
            .header("X-App-Identifier", &uuid)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let text = list_resp.error_for_status()?.text().await?;
        let picked = parse_wp_list(&text, &hit.plant_number)?;
        let (address, series) = build_schedule(picked);
        // Fall tillbaka på search-hitens address+zip_city om list-svaret
        // saknar dessa fält.
        let final_address = if address.is_empty() {
            if hit.zip_city.is_empty() {
                hit.address.clone()
            } else {
                format!("{}, {}", hit.address, hit.zip_city)
            }
        } else {
            address
        };
        Ok((final_address, series))
    }

    // --- Dispatchers ----------------------------------------------------
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, ProviderError> {
        match &self.cfg.auth {
            Auth::Widget { bearer, app_identifier } => {
                self.widget_search(query, bearer, app_identifier).await
            }
            Auth::MobileApp => self.wp_search(query).await,
        }
    }

    async fn widget_schedule(
        &self,
        street: &str,
        city_hint: Option<&str>,
        bearer: &str,
        app_id: &str,
    ) -> Result<(String, Vec<PickupSeries>), ProviderError> {
        let hits = self.widget_search(street, bearer, app_id).await?;
        let matched = hits
            .iter()
            .find(|h| {
                self.matches_city(&h.zip_city)
                    && h.address.eq_ignore_ascii_case(street)
                    && city_hint
                        .map(|c| h.zip_city.to_lowercase() == c.to_lowercase())
                        .unwrap_or(true)
            })
            .or_else(|| {
                hits.iter().find(|h| {
                    self.matches_city(&h.zip_city) && h.address.eq_ignore_ascii_case(street)
                })
            })
            .or_else(|| hits.iter().find(|h| self.matches_city(&h.zip_city)));
        let Some(hit) = matched else {
            return Ok((String::new(), Vec::new()));
        };
        let resp = self.widget_fetch_pickup(&hit.plant_number, bearer, app_id).await?;
        let (address, series) = build_schedule(resp);
        let final_address = if address.is_empty() {
            if hit.zip_city.is_empty() {
                hit.address.clone()
            } else {
                format!("{}, {}", hit.address, hit.zip_city)
            }
        } else {
            address
        };
        Ok((final_address, series))
    }

    // 401 mot widget-läget → varna högt så att någon lyfter en färsk
    // Bearer + X-App-Identifier ur widgetens JS-bundle. Sedan
    // implementationen skrevs (2024-Q4) har AÅS-nyckeln legat orörd i
    // Wayback ~19 månader; en rotation är sällsynt men märks direkt när
    // den händer.
    async fn check_auth(
        &self,
        resp: reqwest::Response,
    ) -> Result<reqwest::Response, ProviderError> {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                target: "avfallsappen",
                provider = self.cfg.id,
                tenant = self.cfg.tenant,
                "upstream 401 — widget bearer/X-App-Identifier likely rotated; \
                 refresh from the current widget JS on the kommunsajt"
            );
        }
        Ok(resp.error_for_status()?)
    }
}

#[derive(Debug, Clone)]
struct SearchHit {
    address: String,
    zip_city: String,
    plant_number: String,
}

// -- Widget-search-parser -----------------------------------------------
fn parse_widget_search(body: &str) -> Result<Vec<SearchHit>, ProviderError> {
    // Response shape är en map från postort → array av träffar. Tom
    // träfflista serialiseras som `[]` på toppnivå — acceptera båda.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Resp {
        Map(BTreeMap<String, Vec<RawHit>>),
        #[allow(dead_code)]
        Empty(Vec<serde::de::IgnoredAny>),
    }
    #[derive(Deserialize)]
    struct RawHit {
        #[serde(default)]
        address: String,
        #[serde(default)]
        zip_city: String,
        #[serde(default)]
        plant_number: String,
    }
    let parsed: Resp = serde_json::from_str(body)?;
    let mut out = Vec::new();
    if let Resp::Map(m) = parsed {
        for (_bucket, hits) in m {
            for h in hits {
                if h.address.is_empty() || h.plant_number.is_empty() {
                    continue;
                }
                out.push(SearchHit {
                    address: h.address,
                    zip_city: h.zip_city,
                    plant_number: h.plant_number,
                });
            }
        }
    }
    Ok(out)
}

fn strip_zip_prefix(s: &str) -> String {
    // Matchar "NNN NN <ort>" eller "NNNNN <ort>" och plockar bort prefixet.
    let trimmed = s.trim();
    let mut prefix_end = 0;
    let mut digits = 0;
    let bytes = trimmed.as_bytes();
    while prefix_end < bytes.len() {
        let c = bytes[prefix_end] as char;
        if c.is_ascii_digit() {
            digits += 1;
            prefix_end += 1;
        } else if c == ' ' && digits > 0 && digits < 5 {
            prefix_end += 1;
        } else {
            break;
        }
    }
    // Kräv exakt 5 siffror totalt (postnummer) för att strippa.
    let stripped_prefix = &trimmed[..prefix_end];
    let digit_count = stripped_prefix.chars().filter(|c| c.is_ascii_digit()).count();
    if digit_count == 5 {
        let rest = trimmed[prefix_end..].trim_start();
        if !rest.is_empty() {
            return rest.to_string();
        }
    }
    trimmed.to_string()
}

// -- Mobile-app-list-parser --------------------------------------------
fn parse_wp_list(body: &str, plant_number: &str) -> Result<PickupResp, ProviderError> {
    // `/list` returnerar array av alla adresser bundna till device-UUID:t.
    // Eftersom `wp_fetch_pickup` registrerar en fräsch UUID per anrop
    // förväntas listan innehålla exakt ett element. Föredra ändå exakt
    // match på topp-nivå-`plant_id` (DVA, Motala, Vallentuna returnerar
    // det) eller på `bins[].plant_number` (Söderköping har inte
    // topp-nivå-plant_id, bara per-bin). Fall tillbaka på första item
    // som säkerhetsnät.
    #[derive(Deserialize)]
    struct RawItem {
        #[serde(default)]
        address: String,
        #[serde(default)]
        city: String,
        #[serde(default)]
        plant_id: String,
        #[serde(default)]
        bins: Vec<RawBin>,
    }
    #[derive(Deserialize)]
    struct RawBin {
        #[serde(default, rename = "type")]
        ty: String,
        #[serde(default)]
        pickup_date: String,
        #[serde(default)]
        plant_number: String,
    }
    let items: Vec<RawItem> = serde_json::from_str(body)?;
    let picked = items
        .iter()
        .position(|i| i.plant_id == plant_number)
        .or_else(|| {
            items.iter().position(|i| {
                i.bins.iter().any(|b| b.plant_number == plant_number)
            })
        })
        .or_else(|| if items.is_empty() { None } else { Some(0) });
    let mut items = items;
    let picked = picked
        .map(|idx| items.swap_remove(idx))
        .unwrap_or_else(|| RawItem {
            address: String::new(),
            city: String::new(),
            plant_id: String::new(),
            bins: Vec::new(),
        });
    Ok(PickupResp {
        address: picked.address,
        city: picked.city,
        bins: picked
            .bins
            .into_iter()
            .map(|b| Bin {
                ty: b.ty,
                pickup_date: b.pickup_date,
            })
            .collect(),
    })
}

#[derive(Deserialize)]
struct PickupResp {
    #[serde(default)]
    address: String,
    #[serde(default)]
    city: String,
    #[serde(default)]
    bins: Vec<Bin>,
}

#[derive(Deserialize)]
struct Bin {
    #[serde(default, rename = "type")]
    ty: String,
    #[serde(default)]
    pickup_date: String,
}

fn build_schedule(resp: PickupResp) -> (String, Vec<PickupSeries>) {
    let address = match (resp.address.trim(), resp.city.trim()) {
        ("", "") => String::new(),
        (a, "") => a.to_string(),
        ("", c) => c.to_string(),
        (a, c) => format!("{a}, {c}"),
    };

    // Gruppera per fraktion; dedup datum (samma tömning kan förekomma i
    // multipla svar).
    let mut by_type: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for bin in resp.bins {
        if bin.ty.is_empty() {
            continue;
        }
        let Ok(date) = NaiveDate::parse_from_str(&bin.pickup_date, "%Y-%m-%d") else {
            continue;
        };
        by_type.entry(bin.ty).or_default().push(date);
    }

    let series = by_type
        .into_iter()
        .map(|(waste_type, mut dates)| {
            dates.sort();
            dates.dedup();
            PickupSeries {
                waste_type,
                frequency_text: String::new(),
                interval_weeks: None,
                anchor: dates,
            }
        })
        .collect();
    (address, series)
}

fn split_input(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    match trimmed.split_once(',') {
        Some((street, city)) => (
            street.trim().to_string(),
            {
                let c = city.trim();
                if c.is_empty() {
                    None
                } else {
                    // Strippa ev. postnummer-prefix så city-hint blir en
                    // rent ortnamn.
                    Some(strip_zip_prefix(c))
                }
            },
        ),
        None => (trimmed.to_string(), None),
    }
}

#[async_trait]
impl Provider for Avfallsappen {
    fn id(&self) -> &'static str {
        self.cfg.id
    }
    fn name(&self) -> &'static str {
        self.cfg.name
    }
    fn placeholder(&self) -> &'static str {
        self.cfg.placeholder
    }
    fn note(&self) -> &'static str {
        self.cfg.note
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let hits = self.search(q).await?;
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for hit in hits {
            if !self.matches_city(&hit.zip_city) {
                continue;
            }
            let value = if hit.zip_city.is_empty() {
                hit.address
            } else {
                format!("{}, {}", hit.address, hit.zip_city)
            };
            if seen.insert(value.clone()) {
                out.push(Suggestion { value });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let (street, city_hint) = split_input(address);
        if street.is_empty() {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        }
        let (out_address, series) = match &self.cfg.auth {
            Auth::Widget { bearer, app_identifier } => {
                self.widget_schedule(&street, city_hint.as_deref(), bearer, app_identifier)
                    .await?
            }
            Auth::MobileApp => {
                self.wp_schedule_atomic(&street, city_hint.as_deref()).await?
            }
        };
        if series.is_empty() && out_address.is_empty() {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        }
        Ok(PickupSchedule {
            address: out_address,
            series,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_widget(cities: &'static [&'static str]) -> Config {
        Config {
            id: "test",
            name: "Test",
            placeholder: "",
            note: "",
            tenant: "example",
            auth: Auth::Widget {
                bearer: "token",
                app_identifier: "uuid",
            },
            cities,
        }
    }

    fn cfg_wp(cities: &'static [&'static str]) -> Config {
        Config {
            id: "test",
            name: "Test",
            placeholder: "",
            note: "",
            tenant: "example",
            auth: Auth::MobileApp,
            cities,
        }
    }


    // -- Widget-search-parser ---------------------------------------
    #[test]
    fn parse_widget_search_groups_and_flattens() {
        let body = r#"{
            "Falköping":[
                {"address":"Storgatan 10","zip_city":"Falköping",
                 "plant_number":"ENC1","type":"Sophämtning","addressType":"next-pickup"}
            ],
            "Floby":[
                {"address":"Storgatan 1","zip_city":"Floby",
                 "plant_number":"ENC2","type":"Sophämtning","addressType":"next-pickup"}
            ]
        }"#;
        let hits = parse_widget_search(body).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits
            .iter()
            .any(|h| h.address == "Storgatan 10" && h.zip_city == "Falköping"));
        assert!(hits
            .iter()
            .any(|h| h.address == "Storgatan 1" && h.zip_city == "Floby"));
    }

    #[test]
    fn parse_widget_search_drops_hits_without_plant_number() {
        let body = r#"{"Skövde":[
            {"address":"","zip_city":"Skövde","plant_number":"X"},
            {"address":"Storgatan 1","zip_city":"Skövde","plant_number":""}
        ]}"#;
        assert!(parse_widget_search(body).unwrap().is_empty());
    }

    #[test]
    fn parse_widget_search_handles_empty_array_response() {
        assert!(parse_widget_search("[]").unwrap().is_empty());
    }

    // -- WP list-parser ---------------------------------------------
    // Mobile-app-mode-testerna gate:as som `#[ignore]` — de rör bara
    // parser-fixturer utan nätverk men följer samma opt-in-mönster som
    // själva feature-toggeln. Kör dem med
    // `cargo test -- --include-ignored` eller
    // `SOPOR_AVFALLSAPPEN_MOBILE=1 cargo test -- --include-ignored`.

    #[test]
    #[ignore = "gated: cargo test -- --include-ignored (SOPOR_AVFALLSAPPEN_MOBILE)"]
    fn parse_wp_list_picks_matching_top_level_plant_id() {
        // DVA/Motala/Vallentuna shape.
        let body = r#"[
            {"address":"Storgatan 1","city":"Insjön","plant_id":"OTHER",
             "bins":[{"type":"Plast","pickup_date":"2026-10-06"}]},
            {"address":"Bagarbyvägen 1","city":"Insjön","plant_id":"WANTED",
             "bins":[{"type":"Matavfall","pickup_date":"2026-09-14"},
                     {"type":"Restavfall","pickup_date":"2026-09-15"}]}
        ]"#;
        let resp = parse_wp_list(body, "WANTED").unwrap();
        assert_eq!(resp.address, "Bagarbyvägen 1");
        assert_eq!(resp.bins.len(), 2);
        assert!(resp.bins.iter().any(|b| b.ty == "Matavfall"));
    }

    #[test]
    #[ignore = "gated: cargo test -- --include-ignored (SOPOR_AVFALLSAPPEN_MOBILE)"]
    fn parse_wp_list_matches_via_bin_plant_number_when_top_level_absent() {
        // Söderköping shape — plant_id saknas topp-nivå, ligger i bins.
        let body = r#"[
            {"address":"Storgatan 1","city":"Söderköping","bins":[
                {"type":"","plant_number":"0073015","pickup_date":"2026-09-14"},
                {"type":"Färgat Glas","plant_number":"0073015","pickup_date":"2026-09-23"}
            ]}
        ]"#;
        let resp = parse_wp_list(body, "0073015").unwrap();
        assert_eq!(resp.address, "Storgatan 1");
        assert_eq!(resp.bins.len(), 2);
    }

    #[test]
    #[ignore = "gated: cargo test -- --include-ignored (SOPOR_AVFALLSAPPEN_MOBILE)"]
    fn parse_wp_list_falls_back_to_first_item_when_ids_dont_match() {
        // Fresh-UUID-flödet: exakt ett item, plant_id kan saknas eller
        // avvika från vår sök-plant. Ta det ändå.
        let body = r#"[{"address":"X","city":"Y","bins":[
            {"type":"Restavfall","pickup_date":"2026-10-01"}
        ]}]"#;
        let resp = parse_wp_list(body, "not-matching-anything").unwrap();
        assert_eq!(resp.address, "X");
        assert_eq!(resp.bins.len(), 1);
    }

    #[test]
    #[ignore = "gated: cargo test -- --include-ignored (SOPOR_AVFALLSAPPEN_MOBILE)"]
    fn parse_wp_list_returns_empty_when_no_items() {
        let resp = parse_wp_list("[]", "MISSING").unwrap();
        assert_eq!(resp.address, "");
        assert!(resp.bins.is_empty());
    }

    #[test]
    #[ignore = "gated: cargo test -- --include-ignored (SOPOR_AVFALLSAPPEN_MOBILE)"]
    fn city_filter_matches_stripped_zip_orter_wp() {
        let p = Avfallsappen::new(reqwest::Client::new(), cfg_wp(&["Insjön", "Leksand"]));
        // Efter parse_wp_search har prefixet redan strippats — mata in ort direkt.
        assert!(p.matches_city("Insjön"));
        assert!(!p.matches_city("Falun"));
    }

    // -- build_schedule (delas mellan lägena) -----------------------
    #[test]
    fn build_schedule_groups_and_dedups_bins() {
        // Götene widget-example — två identiska Brännbart-poster samma
        // datum.
        let resp: PickupResp = serde_json::from_str(
            r#"{"address":"Skolgatan 1","city":"Götene","bins":[
                {"type":"Brännbart","pickup_date":"2026-09-21"},
                {"type":"Brännbart","pickup_date":"2026-09-21"},
                {"type":"Matavfall","pickup_date":"2026-09-14"}
            ]}"#,
        )
        .unwrap();
        let (addr, series) = build_schedule(resp);
        assert_eq!(addr, "Skolgatan 1, Götene");
        assert_eq!(series.len(), 2);
        let bra = series.iter().find(|s| s.waste_type == "Brännbart").unwrap();
        assert_eq!(bra.anchor.len(), 1);
        assert!(bra.interval_weeks.is_none());
        let mat = series.iter().find(|s| s.waste_type == "Matavfall").unwrap();
        assert_eq!(mat.anchor[0], NaiveDate::from_ymd_opt(2026, 9, 14).unwrap());
    }

    #[test]
    fn build_schedule_ignores_invalid_dates_and_empty_types() {
        let resp: PickupResp = serde_json::from_str(
            r#"{"address":"X","city":"Y","bins":[
                {"type":"","pickup_date":"2026-09-15"},
                {"type":"Brännbart","pickup_date":""},
                {"type":"Brännbart","pickup_date":"not-a-date"},
                {"type":"Matavfall","pickup_date":"2026-09-15"}
            ]}"#,
        )
        .unwrap();
        let (_, series) = build_schedule(resp);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].waste_type, "Matavfall");
    }

    // -- Övrigt -----------------------------------------------------
    #[test]
    fn city_filter_case_insensitive_unicode() {
        let p = Avfallsappen::new(reqwest::Client::new(), cfg_widget(&["Skövde", "Timmersdala"]));
        assert!(p.matches_city("skövde"));
        assert!(p.matches_city("SKÖVDE"));
        assert!(p.matches_city("Timmersdala"));
        assert!(!p.matches_city("Falköping"));
    }

    #[test]
    fn split_input_strips_zip_prefix_in_city_hint() {
        let (s, c) = split_input("Bagarbyvägen 1, 793 40 Insjön");
        assert_eq!(s, "Bagarbyvägen 1");
        assert_eq!(c.as_deref(), Some("Insjön"));
    }

    #[test]
    fn split_input_full_form() {
        let (s, c) = split_input("Storgatan 10, Falköping");
        assert_eq!(s, "Storgatan 10");
        assert_eq!(c.as_deref(), Some("Falköping"));
    }

    #[test]
    fn split_input_street_only() {
        let (s, c) = split_input("Storgatan 10");
        assert_eq!(s, "Storgatan 10");
        assert!(c.is_none());
    }

    #[test]
    fn split_input_trailing_comma_no_city() {
        let (s, c) = split_input("Storgatan 10,");
        assert_eq!(s, "Storgatan 10");
        assert!(c.is_none());
    }

    #[test]
    fn strip_zip_prefix_variants() {
        assert_eq!(strip_zip_prefix("793 40 Insjön"), "Insjön");
        assert_eq!(strip_zip_prefix("79340 Insjön"), "Insjön");
        assert_eq!(strip_zip_prefix("Insjön"), "Insjön");
        assert_eq!(strip_zip_prefix("795 30 Rättvik"), "Rättvik");
        // Inte ett postnummer — lämna oförändrat.
        assert_eq!(strip_zip_prefix("12 Storgatan"), "12 Storgatan");
    }
}
