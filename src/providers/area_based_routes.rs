//! Statiska ruttabeller för kommuner som publicerar sina
//! sophämtningsscheman som slingor per veckodag + parity (utan
//! adress-uppslag). Se `area_based.rs` för Provider-koden; detta är
//! bara datat, transkriberat från respektive kommuns webbsida.
//!
//! Alla routes kommer från publika källor (kommunernas egna sidor)
//! och har källa + verifieringsdatum i kommentar per kommun. Special-
//! varianter (månadstömning, sommaravtal, mixade frekvenser) hanteras
//! separat — se noterna per kommun.

use chrono::Weekday;

use super::area_based::{Config, Interval, Parity, Route};

const NOTE_BASE: &str = "Sophämtningsdata via statiskt rutt-schema från kommunen. \
                         Välj din by/område eller din slinga direkt.";

// ---------------------------------------------------------------------------
// Ydre — 10 slingor. Källa: ydre.se/.../sophamtning (verifierad 2026-09-12).
// Utförare: Ydre Åkeri AB. Jämn vecka = mån–fre (5 rutter), udda vecka = mån–fre (5 rutter).
// ---------------------------------------------------------------------------
static YDRE_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Asby", "Hestra (centralorten)"] },
    Route { slinga: "2", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Olstorp", "Indianstigen"] },
    Route { slinga: "3", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Österbymo (landsbygd)", "Hestra (landsbygd)", "Holkåsa", "Ramfall",
            "Sund", "Tranberga", "Brostorp", "Linnekulla"] },
    Route { slinga: "4", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Torpa", "Torpön"] },
    Route { slinga: "5", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Rydsnäs (centralorten)"] },
    Route { slinga: "6", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Österbymo (centralorten)"] },
    Route { slinga: "7", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Asby Udde", "Graby", "Tullerum", "Rönnäs"] },
    Route { slinga: "8", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Grindsbo", "Svinhult", "Axebo"] },
    Route { slinga: "9", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Norra Vi", "Svenningeby"] },
    Route { slinga: "10", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Österbymo (landsbygd)", "Rydsnäs (utmed väg 134)", "Forsnäs",
            "Lägern", "Brokabo", "Vena"] },
];

// ---------------------------------------------------------------------------
// Sorsele — 5 slingor, alla jämna veckor varannan (dvs var 2 vecka). Källa:
// sorsele.se/.../haemtningsrutiner (verifierad 2026-09-12). Matavfall +
// restavfall samma tur (tvådelat kärl).
// ---------------------------------------------------------------------------
static SORSELE_ROUTES: &[Route] = &[
    Route { slinga: "Måndag", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Bockträsk", "Gränsgård", "Granliden", "Storbränna", "Kåtaliden",
            "Buresjön", "Heden", "Fjällnäs", "Fjällnäsvägen", "Forsnäs", "Norrsele",
            "Skansnäs", "Johannisberg", "Högbränna"] },
    Route { slinga: "Tisdag", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Ammarnäsvägen", "Ammarnäs", "Klippen"] },
    Route { slinga: "Onsdag", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Sorsele samhälle", "Gustavsberg", "Färjstället", "Rankbäcken", "Stridsmark"] },
    Route { slinga: "Torsdag", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Gargnäs", "Råstrand", "Tväråträsk", "Lomsele", "Sandsjön", "Sandsele",
            "Staggträsk", "Blattnicksele"] },
    Route { slinga: "Fredag", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Fjällsjönäs", "Skirknäs", "Jiltjaur", "Flakaträsk", "Aha", "Krutträsk",
            "Övre Saxnäs", "Stensund", "Nedre Saxnäs", "Kvarnbränna", "Rågoliden"] },
];

// ---------------------------------------------------------------------------
// Norsjö — 7 slingor med djur-koder. Källa: norsjo.se/.../nar-hamtas-mitt-avfall
// (verifierad 2026-09-12). Del av "Trepartens" nya insamlingssystem
// (Malå/Norsjö/Sorsele).
// ---------------------------------------------------------------------------
static NORSJO_ROUTES: &[Route] = &[
    Route { slinga: "Lämmeln", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Klockarbacken", "Kronan", "Skogsvägen", "Lilltjärnbrännan",
            "Storgatan norr om Kronan", "Sörbyn", "Holktjärn", "Bjursele", "Tjärnliden",
            "Pjäsörn", "Rönnfälla", "Hemmingen", "Arnberg"] },
    Route { slinga: "Sorken", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Norsjö samhälle (exkl. Klockarbacken, Kronan, Lilltjärnbrännan, Sörbyn)"] },
    Route { slinga: "Lodjuret", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Avaliden", "Bränngård", "Talliden", "Risliden", "Kattistjärn", "Kattisberg",
            "Träskliden", "Nyberg", "Anderstjärn", "Kvarnåsen", "Granliden", "Raggsjö",
            "Lillraggsjö", "Brinken", "Gårdkläppen", "Norsjövallen", "Finnäs", "Björknäs",
            "Bodan", "Näset", "Djupnäs", "Kläppen", "Berget"] },
    Route { slinga: "Haren", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Norrby", "Storliden", "Heden", "Ulriksdal", "Brännliden", "Lidbränna",
            "Gissträsk", "Lillholmträsk", "Nybränna", "Norrbränna", "Åmliden",
            "V:a Högkulla", "Gransjö", "Brännland", "Kvammarn", "Mellanå", "Storsele",
            "Bäverhult", "Fromheden", "Lustigkulla"] },
    Route { slinga: "Örnen", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bastuträsk", "Bastuträsk by", "Myrberg", "Ristjälen", "Risberg", "Torpet",
            "Fraukärlen", "Svartnäs", "Kattisträsk", "Granström", "Långvattnet", "Karlsmyr"] },
    Route { slinga: "Rådjuret", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Kvavisträsk", "Böle", "Svanheden", "Rålund", "Gumboda", "Flarken",
            "Kusfors", "Petiknäs", "Sunnanå", "Mörttjärn", "Rengård", "S Kusfors",
            "Båtfors", "Åsen", "Braxträsk", "Holmträsk", "Vikborg", "Lövlund", "Långträsk"] },
    Route { slinga: "Ugglan", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Svansele", "Petikträsk", "Rörträsk", "Ljusliden", "Fågelliden", "Dragnäs",
            "Edebo", "Granbergsliden", "Rörås", "Lillträsk", "Nicknoret", "Mensträsk",
            "Södra Mensträsk", "Finnliden", "Ö:a Högkulla", "Örträsk", "Vargfors",
            "Bjurfors", "Bjurträsk", "Bodberg", "Bastutjärn", "Långmyrliden"] },
];

// ---------------------------------------------------------------------------
// Bjurholm — 2 fredag-slingor (udda/jämn) + Nässund via Nordmalings onsdag udda.
// Källa: bjurholm.se/.../korschema-sophamtning + PDF (verifierad 2026-09-12).
// Grön (hushållsavfall) och brun (matavfall) hämtas samtidigt.
// ---------------------------------------------------------------------------
static BJURHOLM_ROUTES: &[Route] = &[
    Route { slinga: "Fredag udda", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Abborrtjärn", "Backfors", "Backgatan", "Balfors", "Balsjö", "Balåker",
            "Balåliden", "Berglundavägen", "Bjurbäck", "Bjurvattnet", "Bondegatan",
            "Bredträsk", "Brännavägen", "Bäverstigen", "Emmagränd", "Färgargatan",
            "Garvargatan", "Grannäs", "Grönalundsvägen", "Grönåker", "Gustav Jansvägen",
            "Hantverkargatan", "Högås", "Industrivägen", "Jakob Jonsvägen",
            "Kamrersvägen", "Karlsbäck", "Klockarvägen", "Kyrkogatan", "Kyrktjärn",
            "Köpmannagatan", "Lillvägen", "Lill-Vännäs", "Ljusåker", "Mjösjöby",
            "Mellanå", "Nedre Nyland", "Nordansjö", "Nordås", "Norrby 8", "Norrnäs",
            "Nylunda", "Nylidvägen", "Nyåsvägen", "Parkgatan", "Promenaden",
            "Ravingatan", "Rektorsgatan", "Ringvägen", "Sjömyrvägen", "Sjönäs",
            "Skolgatan", "Smedvägen", "Stenskatavägen", "Stennäs", "Storgatan",
            "Stångsjön", "Trädgårdsgatan", "Tvärgränd", "Tobiero", "Villavägen",
            "Vännäsvägen", "Västernyliden", "Västra Strömåker", "Åkervägen",
            "Örträskvägen", "Övre Nyland", "Östergatan"] },
    Route { slinga: "Fredag jämn", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Abborrfors", "Agnäs", "Bastuträsk", "Björknäs", "Braxele", "Brån",
            "Brännland", "Degernäs", "Gravfors", "Högland", "Hörnäs", "Inre Sunnanå",
            "Inre Öreström", "Johanneslund", "Karlsborg", "Lillarmsjö", "Lågsjö",
            "Mellantjärn", "Malmby", "Mariebäck", "Mellansjö", "Norrby 7-20",
            "Nyby 10-62", "Nytorp", "Näsland", "Näsmark", "Otternäs", "Ottervattnet",
            "Pettersborg", "Provåker", "Slättmark", "Solberg", "Stensvattnet",
            "Storarmsjö", "Ström", "Strömsund", "Sunnanå", "Sörfors", "Tällvattnet",
            "Vitvattnet", "Västansjö", "Västerås", "Västomån", "Västra Braxele",
            "Åkernäs", "Åkerslund", "Älskanäs", "Önskanäs", "Öreborg", "Öreström",
            "Östra Strömåker", "Östervik"] },
    Route { slinga: "Nässund", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Nässund (undantag: går på Nordmalings tur)"] },
];

// ---------------------------------------------------------------------------
// Malå — 6 slingor. Källa: mala.se/.../hamtning-av-hushallsavfall (verifierad
// 2026-09-12). Del av "Trepartens" nya insamlingssystem (rest+mat samma tur).
// ---------------------------------------------------------------------------
static MALA_ROUTES: &[Route] = &[
    Route { slinga: "Måndag udda", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Malå tätort norr om Storgatan (exkl. Nyhamn och norrut)"] },
    Route { slinga: "Tisdag udda", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Norra Malånäs", "Tjärnberg", "Adak", "Hundberg", "Björkland",
            "Kuorbevare", "Kokträsk", "Lönås", "Bastusel", "Ljungby", "Öberg",
            "Jakobslund", "Malåliden"] },
    Route { slinga: "Onsdag udda", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Hyreshus och industrin i Malå tätort"] },
    Route { slinga: "Torsdag udda", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Malå tätort vid/söder om Storgatan", "Backgatan", "Bolagsgatan",
            "Rönngatan", "Ytterberg", "Fårträsk", "Svedjan", "Släppträsk", "Svanvik",
            "Aspliden", "Björkås", "Rökå", "Bergås", "Brännberg", "Nåda", "Lövberg",
            "Rentjärn"] },
    Route { slinga: "Onsdag jämn", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Industri i Malå tätort", "Sunnanvik", "Näsudden", "Storselet",
            "Lainejaur", "Springliden", "Mörttjärn", "Brännträsk", "Sandfors",
            "Grundträsk", "Näsberg", "Strömfors", "Brunträsk", "Nyhamn"] },
    Route { slinga: "Fredag jämn", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Holmsjö", "Brännäs", "Malå-Vännäs", "Bergnäs", "Hemnäs", "Löparliden",
            "Södra Malånäs"] },
];

// ---------------------------------------------------------------------------
// Åre — 10 slingor. Källa: are.se/.../sophamtningsturer (verifierad 2026-09-12).
// Utförare: Lundstams Återvinning Åre AB. Matavfall + restavfall samma dag.
// ---------------------------------------------------------------------------
static ARE_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Mårdsund", "Hallen", "Brassta", "Ytterocke", "Kvitsle", "Halabacken",
            "Hammarnäs (väg 321)", "Tossberg (E14)", "Offne"] },
    Route { slinga: "2", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Tossön", "Slagsån", "Helgesjövallen", "Hålland", "Mo", "Undersåker",
            "Kläppenvägen", "Brattland"] },
    Route { slinga: "3", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Järpen"] },
    Route { slinga: "4", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Staa", "Enafors", "Handöl", "Ånn", "Sundsvallen", "Häggsjön (E14)"] },
    Route { slinga: "5", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Brattland", "Vik (E14)", "Såå", "Vik (gamla vägen)", "Åre by"] },
    Route { slinga: "6", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Trägsta", "Månsåsen", "Vällviken", "Överhallen", "Åhn", "Sällsjö",
            "Nybyn", "Pizarros", "Ånäset"] },
    Route { slinga: "7", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Älvvågsviken", "Mörsil", "Äggfors", "Ocke", "Mattmar", "Norra Mällbyn",
            "Högåsen", "Rise", "Semlane", "Semlan", "Undersåker (söder om älven)",
            "Vålådalen", "Ottsjö", "Edsåsdalen"] },
    Route { slinga: "8", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bydalen", "Höglekardalen", "Mårsundsbodarna (endast 1 maj–30 nov)"] },
    Route { slinga: "9", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Storvallen", "Storlien", "Järpbyn", "Bonäshamn", "Huså", "Kallsedet",
            "Kolåsen", "Överäng", "Kall"] },
    Route { slinga: "10", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Åre skidbron", "Duved (E14)", "Duvedsbyn", "Åre Hembygdsgården"] },
];

// ---------------------------------------------------------------------------
// Åsele — 6 slingor (SAV). Källa: sav.nu/.../sophamtning (verifierad 2026-09-12).
// Del av delat Åsele+Dorotea-system.
// ---------------------------------------------------------------------------
static ASELE_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Åsele samhälle 1", "ICA", "Åseborg", "Tallmon", "Vårdcentralen",
            "Kommunhuset"] },
    Route { slinga: "2", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Bomsjö", "Gigsele", "Borgsjö", "Trehörningen", "Volmsjö", "Tallsjö",
            "Fjälltuna", "Klippen", "Oxvattnet", "Långvattnet", "Mossavattnet", "Svedjan",
            "Överrissjö", "Ytterrissjö", "Häggsjömon", "Tegelträsk", "Österstrand",
            "Kvällträsk", "Björnavägen", "Åsele samhälle 3", "Norrstrand", "Sörstrand",
            "Blåviken", "Grisbacka", "Wärdshuset", "Åslia", "Ingo"] },
    Route { slinga: "3", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Åsele samhälle 2", "Lillsjönäs", "Älgsjö", "Yxsjö", "Insjö", "Söråsele",
            "Västerled", "Kullerbacka", "Vårdcentralen"] },
    Route { slinga: "4", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Vårdcentralen", "Åseborg", "Abborviken", "Almsele", "Forsvik", "Tensjö",
            "Svartbäcken", "Torvsele", "Torvsjö", "Holmträsk", "Hälla", "Holmstrand",
            "Björksele", "Gafsele", "Pärlström", "Österrnoret", "Västernoret", "Sörnoret"] },
    Route { slinga: "5", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Fredrika", "Lögda", "Baksjöliden", "Baksjöberg", "Norrfors", "Lövås",
            "Nordanås", "Långbäcken", "Lillögda", "Siksjö", "Lövnäs", "Stennäs",
            "Gärdsjönäs", "Vaksjöberg", "Västansjö"] },
    Route { slinga: "6", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Wärdshuset", "ICA", "Åseborg", "Tallmon", "Vårdcentralen", "Kommunhuset"] },
];

// ---------------------------------------------------------------------------
// Dorotea — 4 slingor (SAV). Källa: sav.nu/.../sophamtning (verifierad 2026-09-12).
// Matavfall + restavfall i samma tur.
// ---------------------------------------------------------------------------
static DOROTEA_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Rajastrand", "Högland", "Storbäck", "Risbäck", "Brännåker", "Dabbnäs",
            "Avasjö (Borgafjäll)", "Borga", "Storjola", "Sutme"] },
    Route { slinga: "2", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Dorotea samhälle"] },
    Route { slinga: "3", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Stamsjön", "Hammar", "Visjömon", "Avasjö", "Varpsjö", "Åkerlandet",
            "Dorotea", "Granberget", "Häggås", "Mårdsjö", "Risnäset", "Lajksjö",
            "Svanabyn", "Granåsen", "Lavsjö", "Grynberget", "Bergbacka", "Lomsjö",
            "Forsnäs"] },
    Route { slinga: "4", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Skavåsen", "Ullsjöberg", "Storberget", "Fågelsta", "Rockvattnet",
            "Bellvik", "Nappsjö", "Lövstrand", "Sörstrand", "Arksjö",
            "Västra och östra Ormsjö", "Månsberg", "Barnäs", "Granliden", "Veksjön",
            "Tvåtjärn", "Avaträsk", "Måntorp", "Fjälltuna"] },
];

// ---------------------------------------------------------------------------
// Jokkmokk — 8 biweekly-slingor + 1 vecko-slinga för Jokkmokk tätort fredag.
// Källa: jokkmokkslbc.se/korturer (verifierad 2026-09-12). Utförare: Jokkmokks
// Lastbilscentral. Endast "sopor" (ingen separat matavfallshämtning).
// ---------------------------------------------------------------------------
static JOKKMOKK_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Karats", "Purkijaur", "Kåbdalis"] },
    Route { slinga: "2", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Porjus", "Vaikijaur", "Älvsborg"] },
    Route { slinga: "3", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Jokkmokk Norra (norr om Storgatan)", "Haraudden", "Östansjö"] },
    Route { slinga: "4", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Jokkmokk Södra (söder om Storgatan)", "Gärdan"] },
    Route { slinga: "5", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Vuollerim"] },
    Route { slinga: "6", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Vuollerim (företag och special)", "Porsi", "Murjek", "Övre Kouka",
            "Högträsk", "Norrvik", "Murkisträsk", "Fatjas", "Juggijaur"] },
    Route { slinga: "7", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Randijaur", "Björkholmen", "Tjåmotis", "Kvikkjokk", "Mattisudden"] },
    Route { slinga: "8", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Nyborg", "Vaimat", "Tårrajaur", "Maitum", "Puottaure", "Kroktjärn",
            "Sudok", "Storbacken", "Koskats", "Skällarim"] },
    Route { slinga: "9", weekday: Weekday::Fri, interval: Interval::Weekly, parity: Parity::Even,
        areas: &["Jokkmokk (tätort)"] },
];

// ---------------------------------------------------------------------------
// Årjäng — 10 slingor. Källa: arjang.se/.../Turlista sophämtning 2026.pdf
// (verifierad 2026-09-12).
// ---------------------------------------------------------------------------
static ARJANG_ROUTES: &[Route] = &[
    Route { slinga: "Årjäng södra", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Årjäng södra"] },
    Route { slinga: "Årjäng norra", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Årjäng norra", "Åsebyn", "Tenvik", "Barlingshult"] },
    Route { slinga: "Töcksfors", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Töcksfors", "Lennartsfors"] },
    Route { slinga: "Sillerud 1", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Sillerud 1", "Svensbyn"] },
    Route { slinga: "Sillerud 2", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Sillerud 2"] },
    Route { slinga: "Fågelvik", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Fågelvik", "Hån"] },
    Route { slinga: "Östervallskog", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Östervallskog"] },
    Route { slinga: "Karlanda", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Karlanda"] },
    Route { slinga: "Holmedal", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Holmedal"] },
    Route { slinga: "Blomskog", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Blomskog"] },
];

// ---------------------------------------------------------------------------
// Valdemarsvik — 10 slingor. Källa: valdemarsvik.se/.../aktuellt-hamtningschema
// (verifierad 2026-09-12). Utförare: Suez. Blandat avfall (ingen separat matavfall).
// ---------------------------------------------------------------------------
static VALDEMARSVIK_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Ringarums tätort", "Boda", "Norrby", "Höckerum", "Nysjö",
            "Hyresfastigheter Valdemarsvik", "Majeldsberget", "Folketsparkskullen",
            "Sörby", "Grännäs"] },
    Route { slinga: "2", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Gusums tätort", "Getterö", "Gryt veckohämtning", "Finnebo",
            "Hummelviks varv"] },
    Route { slinga: "3", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Slängen", "Fillingerum", "Sverkersholm", "Åtvidabergsvägen", "Bredal",
            "Gullersbo", "Hjulerum", "Sågsveden", "Säverum", "Mickelsdal", "Häggebo",
            "Vaterloo", "Kömnevik", "Kaggebo", "Vindö", "Örbäcken"] },
    Route { slinga: "4", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Valdemarsviks tätort (exkl. Majeldsberget, Folketsparkskullen, Sörby, Grännäs)",
            "Vittvik", "Käggla", "Stjärnö"] },
    Route { slinga: "5", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Åsvedal", "Kalerum", "Fredriksnäs", "Kopparhult", "L:a Lövvik", "Dala",
            "Bäckermåla", "Koppartorp", "Kallsö", "Åbäcksnäs", "Eknäsvägen", "Lervik",
            "Målma", "Lilla Syltvik", "Stora Syltvik", "Högved", "Gusum",
            "Hummelviks varv"] },
    Route { slinga: "6", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Ringarum veckohämtning", "Valdemarsvik hyresfastigheter mot Sandvik",
            "Vångsten", "Lövudden", "Gryt samhälle", "Snäckevarp", "Fyrudden",
            "Vikeboda", "Solberga", "Danebo", "Hosum", "Lundby", "Breviksnäs",
            "Hummelviks varv", "Åbäcksnäs"] },
    Route { slinga: "7", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Karlskog", "Rödmålen", "Bråta", "Forsum", "Fängebo", "Lilla Gusum",
            "Fänge", "Solhem", "Tostebo", "Leckersbo", "Byngsbo", "Syntorp",
            "Eriksberg", "Svederna"] },
    Route { slinga: "8", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Gållösa", "Vallby", "Snällebo", "Kattedal", "Fågelvik", "Stjärneberg",
            "Bankeböte", "Glo", "Kvädö", "Ekudden", "Bäckaskog", "Draget", "Rosenlund",
            "Kråkvik", "Åsvikelandet", "Torrö", "Lilla Kalvö", "Skeppsgården", "Grötebo",
            "Ålötterna", "Stockkärr", "Östantorp", "Västerum", "Braberg", "Långrådna",
            "Hornsberg", "Lövbo"] },
    Route { slinga: "9", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Holmtebo", "Karsmåla", "Kärnhult", "Skönero", "Häradssätter",
            "Västertryserum", "Skrickerum", "Skårsjö", "Fallingeberg", "Jonsbo",
            "Knappekulla", "Öndal", "Oltorp", "Kårtorp", "Stockebäck", "Ingelsbo",
            "Tryserum", "Segersrum", "Ramm", "Grindsveden", "Bidalen", "Sjögärdet",
            "Valdemarsviks centrum veckohämtning"] },
    Route { slinga: "10", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Näs", "Gusums lantbruk", "Brantsbo", "Jordbro", "Grönstorp", "Birkekärr",
            "Fastebo", "Ängshästhagen", "Kärret", "Hökdalen", "Askedal", "Mossebo",
            "Ämtöholm", "Fifalla", "Björksätter", "Evalund", "Rullerum", "Strand",
            "Sätterbo", "Barnebo", "Hummelviks varv"] },
];

// ---------------------------------------------------------------------------
// Vilhelmina — 10 slingor. Källa: vilhelmina.se/.../avfallshamtning (verifierad
// 2026-09-12). Notera: Område B har månadstömning okt–apr (ej implementerat —
// bara sommar/varannan-veckas-schemat modelleras här).
// ---------------------------------------------------------------------------
static VILHELMINA_ROUTES: &[Route] = &[
    Route { slinga: "Jämn måndag", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Dikanäs", "Matsdal", "Grönfjäll", "Henriksfjäll", "Kittelfjäll",
            "Borkan", "Gielas"] },
    Route { slinga: "Jämn tisdag", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Volgseleby", "Aronsjö", "Storsele", "Bergland", "Daikanvik", "Västansjö",
            "Eriksberg", "Dorris", "Blaikliden", "Stalon", "Fatsjöluspen", "Lövnäs",
            "Strömnäs", "Granliden", "Rönnäs"] },
    Route { slinga: "Jämn onsdag", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Storholmen", "Svannäs", "Karlsbacka", "Djupdal", "Malgonäs", "Laxbäcken",
            "Malgovik", "Skansholm", "Skog", "Mark"] },
    Route { slinga: "Jämn torsdag", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Myra", "Meselefors", "Viktorp", "Dalasjö", "Bäsksele", "Granberget"] },
    Route { slinga: "Udda måndag", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Grytsjö", "Marsliden", "Saxnäs", "Klimpfjäll", "Grundfors"] },
    Route { slinga: "Udda tisdag", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Solliden", "Lövliden", "Lomsjökullen", "Nordansjö", "Siksjönäs",
            "Hornsjö", "Sjöberg", "Fäboberg", "Nästansjö", "Västra Nästansjö",
            "Heligfjäll", "Annelund", "Västanbäck", "Norra Tresund", "Södra Tresund"] },
    Route { slinga: "Udda onsdag", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Siksjöhöjden", "Hacksjö", "Järvsjöby", "Latikberg", "Bäsksjö",
            "Risträsk", "Ulvoberg", "Fianberg", "Strandkullen"] },
    Route { slinga: "Udda torsdag V", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bergbacka", "Volgsjövägen jämna nr", "området nedanför/väster om Volgsjövägen mot Volgsjön"] },
    Route { slinga: "Udda torsdag Ö", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Volgsjövägen ojämna nr", "området nedanför/öster om Volgsjövägen mot Baksjön"] },
    Route { slinga: "Udda fredag", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Hyreshus och verksamheter inom tätorten (Vilhelmina)"] },
];

// ---------------------------------------------------------------------------
// Övertorneå — 7 slingor (kompost varannan vecka). Källa:
// overtornea.se/tekniska/renhallning-och-atervinning/ (verifierad 2026-09-12).
// Not: Brännbara sopor hämtas 1 gång/månad (ej separat modellerat).
// ---------------------------------------------------------------------------
static OVERTORNEA_ROUTES: &[Route] = &[
    Route { slinga: "Måndag udda", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Armasjärvi", "Bäckesta", "Hedenäset", "Luppio", "Matojärvi",
            "Orjasjärvi", "Potila", "Risudden", "Vuomajärvi"] },
    Route { slinga: "Måndag jämn", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Alkullen", "Ekfors", "Ekobyn", "Hirvijärvi", "Kiilisjärvi", "Koutojärvi",
            "Kukasjärvi", "Liehittäjä", "Littiäinen", "Persomajärvi", "Puostijärven-Ylipää",
            "Puostijärvi", "Raitajärvi", "Ruskola", "V. Armasjärvi"] },
    Route { slinga: "Tisdag udda", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Juoksengi", "Lampisenpää", "Neistenkangas", "Pello", "Svanstein",
            "Valkeakoski", "Ylikuittasjärvi"] },
    Route { slinga: "Tisdag jämn", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Haapakylä", "Korva", "Kuivakangas", "Mauno", "Niskanpää", "Poikkijärvi",
            "Soukolojärvi", "Tureholm", "Vanhaniemi", "Vyöni"] },
    Route { slinga: "Onsdag jämn", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Aapua", "Härkäsaadio", "Jomotusjärvi", "Jänkisjärvi", "Kannusjärvi",
            "Kulmungi", "Kuurajärvi", "Kuusijärvi", "Mettäjärvi", "Mukkajärvi",
            "Mukkavaara", "Olkamangi", "Penttäjä", "Pirttijärvi", "Pyhäjärvi",
            "Rantajärvi", "Ruokojärvi", "Siekasjärvi", "Syväjärvi", "Ylinenjärvi"] },
    Route { slinga: "Torsdag jämn", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Turovaara"] },
    Route { slinga: "Torsdag udda", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Övertorneå (tätorten)"] },
];

// ---------------------------------------------------------------------------
// Filipstad — 8 slingor. Källa: filipstad.se/.../hamtningavkommunaltavfall
// (verifierad 2026-09-12). Rest + matavfall samtidigt.
// ---------------------------------------------------------------------------
static FILIPSTAD_ROUTES: &[Route] = &[
    Route { slinga: "Måndag jämn", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Gammelkroppa", "Svartsången", "Torskbäcken", "Saxå", "Lervik",
            "Gåsgruverakan", "Persberg", "Torkhushöjden", "Horrsjön", "Finngårdarna",
            "upp till f.d. viadukten Långban"] },
    Route { slinga: "Måndag udda", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Färnsjöområdet", "nya Åsen", "Nybacka", "Storhöjden", "Östra Vägen",
            "Flyfallet", "Dammshöjden", "Brattforshyttan", "Mosserud", "Brattfors",
            "Pardis", "Västerud", "Forshyttan", "Svartå", "Vitteberg", "Bolhyttan"] },
    Route { slinga: "Tisdag jämn", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Gåsborn", "Örling", "Klockartorp", "Kosundet", "Holmskogen",
            "Älvsjöhyttan", "Skäfthöjden", "Laggen", "Långban", "Rämmen", "Neva",
            "Liljendal", "Ängkärret", "Franstorp", "Djuprämmen", "Mögrevsände",
            "Hökhöjden"] },
    Route { slinga: "Tisdag udda", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Asphyttan", "Liksta", "Kalhyttan", "Trulskullen", "Storbron",
            "gamla Åsen", "Höglunda", "Grundsdal", "sydvästra Filipstad", "Strandkullen",
            "Karlstadvägen", "Blombackavägen", "Radhusgatan"] },
    Route { slinga: "Onsdag jämn", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Nordmark", "Motjärnshyttan", "Bryggan", "Finnshyttan", "Edsholm",
            "Skåltjärn", "Ekhöjden", "Bosjön", "Stöpsjöhyttan", "Grundsjön", "Sandsjön",
            "Brushöjden"] },
    Route { slinga: "Onsdag udda", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Områden längs rv26", "Prästbäcken", "Asphyttan-riktning", "Bredvik",
            "Västra Nykroppa", "Hornkullen", "sydöstra Filipstad", "Sommarro",
            "Myrängen", "Pålandsvägen", "Övre Stensta", "Nyhyttan"] },
    Route { slinga: "Torsdag jämn", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Lesjöfors"] },
    Route { slinga: "Torsdag udda", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Norra Filipstad (exkl. Åsen)", "östra Nykroppa"] },
];

// ---------------------------------------------------------------------------
// Storuman — 10 slingor. Källa: storuman.se/.../hamtning-av-hushallsavfall
// (verifierad 2026-09-12). Not: fritids-schema (var 4:e vecka) är bild-PDF,
// ej implementerat.
// ---------------------------------------------------------------------------
static STORUMAN_ROUTES: &[Route] = &[
    Route { slinga: "Storuman tätort A", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Barkvägen", "Norra Industriområdet", "Bondevägen", "Bultvägen",
            "Falkvägen", "Fiskarvägen", "Furuvägen", "Granvägen", "Gustav Roséns väg",
            "Gångarstigen", "Hammarvägen", "Hantverksgatan", "Holmsundsgränd",
            "Höjdvägen", "Hökvägen", "Industrigatan", "Klintvägen", "Ljungvägen",
            "Löparstigen", "Materialvägen", "Norrlandsgatan", "Parkvägen",
            "Plankvägen", "Ringvägen", "Röbrostigen", "Skolgatan 28",
            "Skrinnarvägen", "Smedsvägen", "Snickarvägen", "Stenselevägen",
            "Storhällavägen", "Svanvägen", "Sågvägen", "Tallstigen", "Timotejvägen",
            "Trädgårdsvägen", "Tvärgatan", "Ugglevägen", "Vallnäsvägen"] },
    Route { slinga: "Storuman tätort B", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Backvägen", "Bergsvägen", "Centralgatan", "Kungavägen", "Lokgränd",
            "Luspgränd", "Luspholmen", "Luspnäsvägen", "Rallargränd", "Skidvägen",
            "Skolgatan 1-24", "Stationsgatan", "Stenselevägen", "Strandgränd",
            "Strandvägen", "Zakrisvägen"] },
    Route { slinga: "Storuman omland mån", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Sandås", "Nordanås", "Slussfors", "Gardsjönäs", "Abborrberg",
            "Danasjö", "Ankarsund", "Strömsund", "Bäckmark", "Blaiken", "Renberg",
            "Laisbäck", "Sördal"] },
    Route { slinga: "Storuman omland tis", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Gunnarberg", "Norrdal"] },
    Route { slinga: "Storuman omland ons", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Forsvik", "Barsele", "Gunnarn", "Juktån", "Östansjö", "Åskiljeby",
            "Åskilje", "Grundfors", "Pauträsk", "Skarvsjöby", "Kaskeluokt"] },
    Route { slinga: "Stensele", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Stensele"] },
    Route { slinga: "Hemavan/Tärnaby mån", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Krokfors", "Laisaliden", "Laisholm", "Laxnäs", "Solberg",
            "Oltokkbäcken", "Klippen", "Umfors", "Umasjö", "Vilasund", "Strimasund",
            "Mjölkbäcken", "Kåtaviken", "Bredviken", "Högstaby", "Riksgränsen"] },
    Route { slinga: "Tärnaby tätort", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Tärnaby", "Granås", "Konäset", "Västansjö", "Portbron", "Hemavan",
            "Tängvattnet", "Rönäs"] },
    Route { slinga: "Tärnaby omland", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Tärnafors", "Bäcknäs", "Storsand", "Sundsliden", "Forsbäck", "Jokksjaure",
            "Mellansjö", "Kråkberg", "Mittibäcken", "Boksjön", "Fansen", "Björkbacken",
            "Siambäcken", "Norra Fjällnäs", "Mosekälla", "Joeström", "Ström", "Joesjö",
            "Gröndal", "Boxfjäll", "Tärnamo", "Ängesdal", "Storsandnäset", "Högås",
            "Fräkenvik", "Lövlund", "Vallenäs", "Södra Sandnäset"] },
    Route { slinga: "Umnäs-slingan", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Stålfjäll", "Yttervik", "Forsmark", "Umnäs", "Silverberg", "Åsvattnet",
            "Harrvik", "Brattåker", "Södra Långvattnet", "Långsjöby", "Volvonäset",
            "Norrberg"] },
];

// ---------------------------------------------------------------------------
// Vännäs — 8 slingor (6 privat-slingor + 2 verksamhets-veckotömning). Källa:
// vannas.se/download/.../Tömningsschema Vännäs.pdf (verifierad 2026-09-12).
// ---------------------------------------------------------------------------
static VANNAS_ROUTES: &[Route] = &[
    Route { slinga: "Vännäs tätort A", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Alvägen", "Bjällerkransen", "Björkvägen", "Brinkgatan", "Dansbanegatan",
            "Envägen", "Fällforsvägen (exkl. 98)", "Fältjägargatan", "Garvaregatan",
            "Granvägen", "Grenvägen", "Göransvägen", "Hagvägen", "Hammargatan",
            "Hantverkargatan", "Hovslagargatan", "Idrottsgatan", "Isbanegatan",
            "Knektgatan", "Korpralsgatan", "Kronvägen", "Kvartsvägen", "Kyrkogatan 3",
            "Liljas väg", "Lägervägen", "Magasinsgatan", "Mikaelsvägen", "Målargatan",
            "Norrlandsgatan", "Nybyvägen", "Paradvägen", "Pengsjövägen", "Prästgatan",
            "Regnbågsgatan", "Rektorsgatan", "Rotvägen", "Skolgatan", "Stamvägen",
            "Stenvägen", "Stinsgatan", "Tallvägen", "Västra Järnvägsgatan", "Åkervägen",
            "Älvdalagatan", "Östra Bangatan"] },
    Route { slinga: "Landsbygd A", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bastumyren", "Berglunda", "Bergnäs", "Björnlandsbäck", "Bojnäs",
            "Dalarö", "Degermyr", "Eriksborg", "Fagernäs", "Fredrikshall", "Hjåggsjö",
            "Holmbäck", "Holmsjö", "Håknäs", "Högbäck", "Högland", "Kallhögen",
            "Karlslund", "Klintsjö", "Kronoborg", "Kvarnsvedjan", "London", "Lybäck",
            "Långsjö", "Långåker", "Marahällan", "Mjösjö", "Mobäck", "Mosjö", "Nybo",
            "Nyborg", "Nyland", "Nylandsnäs", "Nyliden", "Nynäs", "Ockelsjö",
            "Pengbacken", "Penglund (exkl. 150)", "Pengsjö", "Skavdal", "Strömbäck",
            "Sunnanå (exkl. 61)", "Södra Hjåggsjö", "Södra Nyby", "Tobacka",
            "Trinnliden", "Vinbäck", "Vännänget 1", "Västerås", "Åkerbäck", "Örsbäck",
            "Östanbäck", "Östanlid", "Östansjö"] },
    Route { slinga: "Landsbygd B", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Bastumyren", "Berglunda", "Bergnäs", "Björnlandsbäck", "Bojnäs",
            "Dalarö", "Degermyr", "Eriksborg", "Fagernäs", "Fredrikshall", "Hjåggsjö",
            "Holmbäck", "Holmsjö", "Håknäs", "Högbäck", "Högland", "Kallhögen",
            "Karlslund", "Klintsjö", "Kronoborg", "Kvarnsvedjan"] },
    Route { slinga: "Vännäs tätort B", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Bergbäck", "Bergliden", "Brännan", "Brännfors", "Fagerlund", "Forsbacka",
            "Fredsgatan", "Frejgatan", "Fritidsvägen", "Fäbokvarn", "Fällfors",
            "Fällforsbäck", "Fällforsselet", "Fällforsvägen 98", "Gamla Tväråbäck",
            "Gransele", "Gryningsvägen", "Gullbäck", "Gullsjö", "Gullsjönäs",
            "Gullsjöäng", "Gåsgränd", "Harrsele", "Harrseleforsen", "Hednäs", "Hällfors",
            "Hällnäs", "Högås", "Innergård", "Jämteböle", "Järvdal", "Kamparbäck",
            "Karlsberg", "Kommendörsgatan", "Kolksele", "Konduktörsgatan",
            "Konstnärsgatan", "Kungsgatan", "Kvarnfors", "Kyrkogatan (exkl. 3)",
            "Köpmangatan", "Lillsjö", "Lyckselevägen", "Långgatan", "Långnäs",
            "Lärargatan", "Marsgränd", "Mångränd", "Nilsland", "Norra Drottninggatan",
            "Norra Parkgatan", "Norrmalm", "Nygård", "Orrböle", "Pengfors",
            "Penglund 150", "Pilgatan", "Rönnvägen", "Selsberg", "Skymningsvägen",
            "Snålltjärn", "Solgränd", "Stennäs", "Storgatan", "Stjärngatan",
            "Stärkesmark", "Södra Drottninggatan", "Södra Parkgatan", "Tallberg",
            "Thulegatan", "Tjärngatan", "Torpgatan", "Tväråbäck", "Umevägen 8-77",
            "Vegagatan", "Vintergatan", "Västerbäck", "Västra Pengfors",
            "Västra Stärkesmark", "Ytterkolksele", "Ånäset", "Önskanäs", "Örngatan",
            "Östergård", "Österselet", "Östra Järnvägsgatan"] },
    Route { slinga: "Nygatan-slingan", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Garverigränd", "Nygatan 3"] },
    Route { slinga: "Vännäs tätort C", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Andelsvägen", "Blomstervägen", "Brånsvägen", "Ekonomivägen",
            "Fågelstigen", "Företagsvägen", "Gothnellsgatan", "Hamrénsvägen",
            "Hembergsleden", "Hembergsvägen", "Hästhovsvägen", "Konvaljvägen",
            "Kungstensvägen", "Kvarnstigen", "Linnévägen", "Marknadsgränd",
            "Mästaregatan", "Nygatan (exkl. 3)", "Nöjesgatan", "Ringgatan",
            "Rosenvägen", "Rådjursgatan", "Skogsvägen", "Skomakargatan", "Snickargatan",
            "Solrosvägen", "Stationsvägen", "Strandgatan", "Såggatan", "Södra Promenaden",
            "Tvärågränd", "Umevägen 102-232", "Vallmovägen", "Villastigen",
            "Vårdhemsvägen", "Åstigen", "Ängsvägen"] },
    Route { slinga: "Veckotömning", weekday: Weekday::Wed, interval: Interval::Weekly, parity: Parity::Even,
        areas: &["Verksamheter/flerbostadshus (1×/vecka)"] },
];

// ---------------------------------------------------------------------------
// Strömsund — 15 slingor (Härbergsdalen månadsvis är utelämnad). Källa:
// stromsund.se/1566.html (verifierad 2026-09-12).
// ---------------------------------------------------------------------------
static STROMSUND_ROUTES: &[Route] = &[
    Route { slinga: "Hammerdal mån", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bredkälen", "Fagerdal", "Gisselås", "Grenås", "Grenåskilen", "Gåxsjö",
            "Henningskälen", "Kakuåsen", "Klumpen", "Lomåsen", "Länglingen", "Nyland",
            "Raftsjöhöjden", "Röningen", "Sikås", "Sikåskälen", "Yxskaftkälen"] },
    Route { slinga: "Hammerdal tis", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Ede", "Fyrån", "Hammerdals samhälle", "Håxås mot södra Edevägen",
            "Skarpås", "Östersundsvägen mot södra Edevägen"] },
    Route { slinga: "Hammerdal ons", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bye", "Böle", "Ede", "Edefors", "Edevägen", "Fyrsjön", "Fyrås",
            "Gravasund", "Görvik", "Hall-Håxåsen", "Hallviken", "Hälgåkilen", "Kilen",
            "Lorås", "Solberg", "Sörviken", "Tannsjön", "Änge"] },
    Route { slinga: "Backe", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Backe samhälle", "Bofors", "Färjsundet", "Högbränna", "Jansjönoret",
            "Johannesberg", "Mullnäset", "Mårdsjön", "Noret", "Norrby", "Sandviken",
            "Sikselet", "Silsjönäs", "Sporrsjönäs", "Stamsele", "Storhöjden",
            "Storöbodarna", "Tjärnnäset", "Tännviken", "Täxan", "Vågdalen", "Vängel",
            "Ånäset", "Österkälen"] },
    Route { slinga: "Rossön", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bergsjö", "Bergsjöåsen", "Bovattnet", "Buskholm", "Bölen", "Bölesmon",
            "Fjällsjösil", "Grundsjö", "Hocksjö", "Hällnäset", "Hällvattnet", "Hössjön",
            "Jansjö", "Landsomberget", "Lesjötorp", "Lia", "Litjärnberget", "Långåsen",
            "Nagasjön", "Norrnäs", "Näset", "Orrnäs", "Rossön", "Rudsjö", "Sil",
            "Sunnansjö", "Sör-Edsta", "Trångåsen", "Volmvattnet", "Älgön", "Ön",
            "Österåsen"] },
    Route { slinga: "Södra Vattudalen", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bredkälsflon", "Bonäset", "Djupudden", "Draganäs", "Havsnäs",
            "Hillsand", "Holsund", "Häggnäset", "Järvsand", "Kalkberget", "Kärrnäset",
            "Lövberga", "Murunäset", "Nyhamn", "Renålandet", "Svaningen", "Sved",
            "Södra Öhn", "Sörvik", "Vedjeön", "Västanvik", "Västvik", "Älghallen",
            "Öjarn"] },
    Route { slinga: "Gäddede fritids udda", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Bågede", "Framnäs", "Fågelberget", "Gäddede samhälle", "Gussvattnet",
            "Holmvik", "Håkafot", "Risnäs", "Sjulsåsen", "Spännviken", "Storvattnet",
            "Torsfjärden"] },
    Route { slinga: "Gäddede verksamhet udda", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Gäddede samhälle (verksamheter)"] },
    Route { slinga: "Strömsund V", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Samhället väster om Lövbergavägen (ej Strömbacka)"] },
    Route { slinga: "Strömsund Ö", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Samhället öster om Lövbergavägen (inkl. Strömbacka)"] },
    Route { slinga: "Strömsund omland", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Annenäs", "Bygda", "Harbäcken", "Långön", "Nyvik", "Näsviken",
            "området runt SCF Betongelement", "Risselås", "Strand", "Tullingsås",
            "Ulriksfors", "Öhn", "Ösundet"] },
    Route { slinga: "Tåsjö", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Abborrholmberget", "Aldernäset", "Brattbäcken", "Brattremmen",
            "Granbergsbyn", "Granön", "Högnäset", "Karbäcken", "Kyrktåsjö", "Lövvik",
            "Mon", "Norråker", "Rotnäset", "Sandnäset", "Saxåmon", "Skansnäset",
            "Skänknäsberget", "Stornäsudden", "Svedje", "Tjädernäset",
            "Tjärnmyrberget", "Tåsjö", "Tåsjöberget", "Tåsjöedet", "Viken",
            "Västertåsjö", "Östra Havsnäs"] },
    Route { slinga: "Hoting", weekday: Weekday::Fri, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Andersnäset", "Björksjönäs", "Bosundet", "Brocksjön", "Flyn",
            "Hotings samhälle", "Kilvamma", "Lunne", "Rörström", "Sund", "Sundet",
            "Skirsjöedet", "Stornäset", "Valån", "Vike", "vissa efter E45 i Lövberga",
            "Västra Hoting"] },
    Route { slinga: "Gäddede fritids jämn", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Ankarede", "Ankarvattnet", "Björkvattnet", "Blåsjöfallet", "Bränna",
            "Jormlien", "Jormvattnet", "Junsterforsen", "Junsternäs", "Kycklingvattnet",
            "Kyrkbollandet", "Lermon", "Lillien", "Lugnvik", "Långviken", "Mesvattnet",
            "Mittilia", "Mon", "Rolandstorp", "Sandnäset", "Småvattsbränna",
            "Stora Blåsjön", "Stekenjokkvägen", "Sör-Blåsjön", "Vallåsen", "Viken",
            "Vilmon", "Vågen", "Väktarmon"] },
    Route { slinga: "Gäddede verksamhet jämn", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Gäddede", "Junsternäs", "Jorm", "Blåsjön", "Ankarvattnet", "Ankarede",
            "Sandnäset", "Mon", "Bränna", "Björkvattnet"] },
    Route { slinga: "Norra Vattudalen", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Alanäs", "Alavattnet", "Allviken", "Gubbhögen", "Gärdnäs", "Gärdviken",
            "Harrsjön", "Klöva", "Klövsand", "Lidsjöberg", "Lillviken", "Ringvattnet",
            "Siljeåsen", "Äspnäs", "Övre Lillviken"] },
];

// ---------------------------------------------------------------------------
// Ragunda — 8 slingor. Källa: ragunda.se/.../sophamtning.1153.html
// (verifierad 2026-09-12). Gäller nuvarande system t.o.m. 2026-12-31; nytt
// FNI-schema införs 2027-01-01 (matavfall varannan vecka + restavfall var 6:e
// vecka) — providern måste uppdateras då.
// ---------------------------------------------------------------------------
static RAGUNDA_ROUTES: &[Route] = &[
    Route { slinga: "1", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Hammarstrand", "Bispgården (veckoabonnenter)"] },
    Route { slinga: "2", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Betlehem", "Mårdsjön", "Öravattnet", "Skyttmon", "Borgänge", "Borgvattnet",
            "Fullsjön", "Björkvattnet", "Boberg", "Selet", "Selsålandet", "Köttsjön",
            "Överammer", "Färsån", "Ammer", "Mörtsjön", "Stugun (veckoabonnenter)"] },
    Route { slinga: "3", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Fisksjölandet", "Dalbo", "Eriksberg", "Lybäck", "Torvalla", "Sundet",
            "Näverede", "Midskog", "Brynjegård", "Torsgård", "Skogslund", "Kompaniet",
            "Strandbodarna", "Bomsund", "Sättsjön", "Höglunda", "Moarna", "Borglunda",
            "Vågsäter", "Koviken", "Överböle", "Hoo", "Rävanäset"] },
    Route { slinga: "4", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Stugun", "Mörtån", "Nybodarna", "Strånäset", "Fiskviken", "Strömsnäs",
            "Selsviken", "Krångede", "Döviken", "Krokvåg", "Gevåg (2 ggr/v-abonnenter)"] },
    Route { slinga: "5", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Stugun", "Hammarstrand", "Bispgården (veckoabonnenter)"] },
    Route { slinga: "6", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Vikbäcken", "Ragunda", "Näset", "Näsmoarna", "Edesmoarna", "Kilen",
            "Böle", "Västeråsen", "Österåsen", "Reva", "Mjösjön", "Korsåmon", "Hölle",
            "Stadsforsen", "Utanede", "Flomyran", "Fångsjöbacken"] },
    Route { slinga: "7", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Lien", "Västerede", "Sörsjön", "Österede", "Svarthålsforsen",
            "Bispgården", "Bispfors", "Halån", "Vågen", "Trefoten", "Pålgård", "Skogen"] },
    Route { slinga: "8", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Kullsta", "Kånkback", "Hammarstrand (2 ggr/v-abonnenter)"] },
];

// ---------------------------------------------------------------------------
// Pajala — 7 A-områdes-slingor (basbanan, varannan vecka). Källa:
// pajala.se/media/uvyeo2zs/turlista.pdf (verifierad 2026-09-12). Not: B- och
// C-områdenas (glesbygd) var-4:e-vecka-scheman är per-veckonummer-baserade
// och passar inte den enkla "veckodag + parity"-modellen; endast A-områdena
// är implementerade.
// ---------------------------------------------------------------------------
static PAJALA_ROUTES: &[Route] = &[
    Route { slinga: "Tur 1 A", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Hörntorpet", "Kivijärvi", "Korpilombolo", "Kuusilaki", "Limingojärvi",
            "Markusvinsa", "Ohtanajärvi", "Pimpiö", "Pirttiniemi", "Sattajärvi",
            "Suaningi", "Teurajärvi", "Tiehaara (Tallheden)", "Välivaara (Sattajärvi)"] },
    Route { slinga: "Tur 2 A", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Granvik", "Grönbo", "Hukanmaa", "Kaalama", "Kaalamakoski",
            "Keräntöjärvi", "Kihlangi", "Kitkiöjoki", "Kitkiöjärvi", "Kuusiniemi",
            "Lovikka by", "Merasjärvi", "Muodoslompolo", "Muonionalusta", "Parkajoki",
            "Parkalompolo", "Östra Kangos"] },
    Route { slinga: "Tur 3 A", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Pajala"] },
    Route { slinga: "Tur 4 A", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Even,
        areas: &["Autio", "Erkheikki", "Jarhois", "Juhonpieti", "Kardis", "Kassa",
            "Liviöjärvi", "Mertapalo", "Mukkakangas", "Södra Kengis", "Taipalensuu",
            "Torinen", "Tuohmaanniemi"] },
    Route { slinga: "Tur 5 A", weekday: Weekday::Mon, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Kainulasjärvi", "Lahdenpää", "Narken"] },
    Route { slinga: "Tur 6 A", weekday: Weekday::Tue, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Anttis", "Huhtanen", "Junosuando", "Kangos", "Lovikka (väg 395)"] },
    Route { slinga: "Tur 7 A", weekday: Weekday::Wed, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Männikkö", "Peräjävaara", "Tärendö (torsdag udda)"] },
    Route { slinga: "Tur 8 A", weekday: Weekday::Thu, interval: Interval::Biweekly, parity: Parity::Odd,
        areas: &["Aareavaara", "Huuki", "Jupukka", "Kaunisjoensuu", "Kaunisvaara",
            "Kieksiäisvaara", "Kolari", "Niva (Pajala)", "Norra Kengis",
            "Ristimella (Kolari-Huuki)", "Rova (Pajala)", "Sahavaara (Kaunisvaara)",
            "Vittaniemi (Norra Kengis)"] },
];

/// Sammanställning: hela listan över statiska Configs som ska registreras.
/// Placeholder-texten anpassas per kommun för bättre UX; noten är shared
/// eftersom informationen är samma per definition (statiskt schema).
pub const CONFIGS: &[Config] = &[
    Config {
        id: "ydre",
        name: "Ydre",
        placeholder: "t.ex. Asby, Österbymo eller Slinga 3",
        note: NOTE_BASE,
        routes: YDRE_ROUTES,
    },
    Config {
        id: "sorsele",
        name: "Sorsele",
        placeholder: "t.ex. Ammarnäs, Gustavsberg eller Slinga Onsdag",
        note: NOTE_BASE,
        routes: SORSELE_ROUTES,
    },
    Config {
        id: "norsjo",
        name: "Norsjö",
        placeholder: "t.ex. Bastuträsk, Kvavisträsk eller Slinga Örnen",
        note: NOTE_BASE,
        routes: NORSJO_ROUTES,
    },
    Config {
        id: "bjurholm",
        name: "Bjurholm",
        placeholder: "t.ex. Storgatan, Bjurbäck eller Nässund",
        note: NOTE_BASE,
        routes: BJURHOLM_ROUTES,
    },
    Config {
        id: "mala",
        name: "Malå",
        placeholder: "t.ex. Adak, Malåliden eller Storgatan",
        note: NOTE_BASE,
        routes: MALA_ROUTES,
    },
    Config {
        id: "are",
        name: "Åre",
        placeholder: "t.ex. Åre by, Duved eller Vålådalen",
        note: NOTE_BASE,
        routes: ARE_ROUTES,
    },
    Config {
        id: "asele",
        name: "Åsele",
        placeholder: "t.ex. Åsele samhälle, Fredrika eller Torvsjö",
        note: NOTE_BASE,
        routes: ASELE_ROUTES,
    },
    Config {
        id: "dorotea",
        name: "Dorotea",
        placeholder: "t.ex. Dorotea samhälle, Borgafjäll eller Avaträsk",
        note: NOTE_BASE,
        routes: DOROTEA_ROUTES,
    },
    Config {
        id: "jokkmokk",
        name: "Jokkmokk",
        placeholder: "t.ex. Jokkmokk, Vuollerim eller Kvikkjokk",
        note: NOTE_BASE,
        routes: JOKKMOKK_ROUTES,
    },
    Config {
        id: "arjang",
        name: "Årjäng",
        placeholder: "t.ex. Årjäng södra, Töcksfors eller Karlanda",
        note: NOTE_BASE,
        routes: ARJANG_ROUTES,
    },
    Config {
        id: "valdemarsvik",
        name: "Valdemarsvik",
        placeholder: "t.ex. Valdemarsviks tätort, Gusum eller Slinga 5",
        note: NOTE_BASE,
        routes: VALDEMARSVIK_ROUTES,
    },
    Config {
        id: "vilhelmina",
        name: "Vilhelmina",
        placeholder: "t.ex. Vilhelmina tätort, Kittelfjäll eller Saxnäs",
        note: NOTE_BASE,
        routes: VILHELMINA_ROUTES,
    },
    Config {
        id: "overtornea",
        name: "Övertorneå",
        placeholder: "t.ex. Övertorneå, Hedenäset eller Juoksengi",
        note: NOTE_BASE,
        routes: OVERTORNEA_ROUTES,
    },
    Config {
        id: "filipstad",
        name: "Filipstad",
        placeholder: "t.ex. Nordmark, Lesjöfors eller Persberg",
        note: NOTE_BASE,
        routes: FILIPSTAD_ROUTES,
    },
    Config {
        id: "storuman",
        name: "Storuman",
        placeholder: "t.ex. Storuman, Hemavan eller Tärnaby",
        note: NOTE_BASE,
        routes: STORUMAN_ROUTES,
    },
    Config {
        id: "vannas",
        name: "Vännäs",
        placeholder: "t.ex. Vännäs tätort, Tväråbäck eller Storgatan",
        note: NOTE_BASE,
        routes: VANNAS_ROUTES,
    },
    Config {
        id: "stromsund",
        name: "Strömsund",
        placeholder: "t.ex. Strömsund, Gäddede, Hoting eller Hammerdal",
        note: NOTE_BASE,
        routes: STROMSUND_ROUTES,
    },
    Config {
        id: "ragunda",
        name: "Ragunda",
        placeholder: "t.ex. Hammarstrand, Bispgården eller Stugun",
        note: "Statiskt rutt-schema gäller t.o.m. 2026-12-31. Från 2027-01-01 \
               inför Ragunda FNI (fastighetsnära insamling) — schemat i denna \
               feed kan behöva uppdateras då.",
        routes: RAGUNDA_ROUTES,
    },
    Config {
        id: "pajala",
        name: "Pajala",
        placeholder: "t.ex. Pajala, Junosuando eller Tärendö",
        note: "Statiskt rutt-schema för A-områden (varannan vecka). B- och \
               C-områdenas glesbygdsscheman (var 4:e vecka med specifika \
               veckonummer) är inte implementerade — se pajala.se om du bor \
               utanför A-området.",
        routes: PAJALA_ROUTES,
    },
];
