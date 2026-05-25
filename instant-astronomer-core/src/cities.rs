//! # Geolocation and City Database System
//!
//! This module implements a robust, lightweight in-memory city database.
//! To ensure zero-config compilation and complete cross-platform portability
//! (especially for `wasm32-unknown-unknown` where linking native C libraries like
//! SQLite can be complex), it embeds a catalog of major worldwide cities.
//!
//! It provides both prefix search (simulating FTS5) and spelling-insensitive phonetic
//! search using the Soundex algorithm, as specified in the implementation design.

/// Representation of a geographical city entity.
///
/// All string fields are `&'static str` so the bundled catalog can live in a
/// `const` table (matches the static-asset model in section 3.1 of
/// `implementation.md` without requiring runtime allocation).
#[derive(Debug, Clone, Copy)]
pub struct City {
    pub name: &'static str,
    pub state: &'static str,
    pub country: &'static str,
    pub country_code: &'static str,
    pub latitude: f64,
    pub longitude: f64,
}

/// Compute the Soundex phonetic code for a string.
/// Soundex maps a name to a 4-character code (e.g., "Denver" -> "D516").
pub fn soundex(input: &str) -> String {
    if input.is_empty() {
        return "0000".to_string();
    }

    let s = input.to_uppercase();
    let mut chars = s.chars();
    let first_char = chars.next().unwrap_or(' ');
    if !first_char.is_alphabetic() {
        return "0000".to_string();
    }

    let mut code = String::new();
    code.push(first_char);

    let get_digit = |c: char| -> Option<char> {
        match c {
            'B' | 'F' | 'P' | 'V' => Some('1'),
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => Some('2'),
            'D' | 'T' => Some('3'),
            'L' => Some('4'),
            'M' | 'N' => Some('5'),
            'R' => Some('6'),
            _ => None,
        }
    };

    let mut last_digit = get_digit(first_char);

    for c in chars {
        if let Some(digit) = get_digit(c) {
            // Adjacent letters with the same code are joined
            if Some(digit) != last_digit {
                code.push(digit);
                last_digit = Some(digit);
                if code.len() == 4 {
                    break;
                }
            }
        } else if c != 'H' && c != 'W' {
            // Non-coded letters (except H and W) break adjacency grouping
            last_digit = None;
        }
    }

    // Pad with zeros if necessary
    while code.len() < 4 {
        code.push('0');
    }

    code
}

/// Curated catalog of cities the prefix + Soundex search runs against.
///
/// Hand-picked rather than ingested from the full
/// `countries-states-cities-database` so the WASM blob stays small
/// (the eventual upgrade — per `implementation.md` §3.1 — is the
/// per-country gzip-CSV asset pipeline, which we haven't built yet).
///
/// Coverage targets:
/// - All US state capitals + the 50 largest US cities (including
///   Irvine — which was missing from the first cut).
/// - Capital cities of every country with > 5M people, plus a
///   handful of culturally / astronomically notable observatories.
/// - Spread across all inhabited longitudes so the projection /
///   timezone math has something sensible to render no matter where
///   the user is.
pub const BUILTIN_CITIES: &[City] = &[
    // ── United States — major metros + state capitals ─────────────────
    City { name: "New York",       state: "New York",        country: "United States", country_code: "US", latitude:  40.7128, longitude:  -74.0060 },
    City { name: "Los Angeles",    state: "California",      country: "United States", country_code: "US", latitude:  34.0522, longitude: -118.2437 },
    City { name: "Chicago",        state: "Illinois",        country: "United States", country_code: "US", latitude:  41.8781, longitude:  -87.6298 },
    City { name: "Houston",        state: "Texas",           country: "United States", country_code: "US", latitude:  29.7604, longitude:  -95.3698 },
    City { name: "Phoenix",        state: "Arizona",         country: "United States", country_code: "US", latitude:  33.4484, longitude: -112.0740 },
    City { name: "Philadelphia",   state: "Pennsylvania",    country: "United States", country_code: "US", latitude:  39.9526, longitude:  -75.1652 },
    City { name: "San Antonio",    state: "Texas",           country: "United States", country_code: "US", latitude:  29.4241, longitude:  -98.4936 },
    City { name: "San Diego",      state: "California",      country: "United States", country_code: "US", latitude:  32.7157, longitude: -117.1611 },
    City { name: "Dallas",         state: "Texas",           country: "United States", country_code: "US", latitude:  32.7767, longitude:  -96.7970 },
    City { name: "San Jose",       state: "California",      country: "United States", country_code: "US", latitude:  37.3382, longitude: -121.8863 },
    City { name: "Austin",         state: "Texas",           country: "United States", country_code: "US", latitude:  30.2672, longitude:  -97.7431 },
    City { name: "Jacksonville",   state: "Florida",         country: "United States", country_code: "US", latitude:  30.3322, longitude:  -81.6557 },
    City { name: "Fort Worth",     state: "Texas",           country: "United States", country_code: "US", latitude:  32.7555, longitude:  -97.3308 },
    City { name: "Columbus",       state: "Ohio",            country: "United States", country_code: "US", latitude:  39.9612, longitude:  -82.9988 },
    City { name: "Charlotte",      state: "North Carolina",  country: "United States", country_code: "US", latitude:  35.2271, longitude:  -80.8431 },
    City { name: "San Francisco",  state: "California",      country: "United States", country_code: "US", latitude:  37.7749, longitude: -122.4194 },
    City { name: "Indianapolis",   state: "Indiana",         country: "United States", country_code: "US", latitude:  39.7684, longitude:  -86.1581 },
    City { name: "Seattle",        state: "Washington",      country: "United States", country_code: "US", latitude:  47.6062, longitude: -122.3321 },
    City { name: "Denver",         state: "Colorado",        country: "United States", country_code: "US", latitude:  39.7392, longitude: -104.9903 },
    City { name: "Washington",     state: "District of Columbia", country: "United States", country_code: "US", latitude:  38.9072, longitude:  -77.0369 },
    City { name: "Boston",         state: "Massachusetts",   country: "United States", country_code: "US", latitude:  42.3601, longitude:  -71.0589 },
    City { name: "El Paso",        state: "Texas",           country: "United States", country_code: "US", latitude:  31.7619, longitude: -106.4850 },
    City { name: "Nashville",      state: "Tennessee",       country: "United States", country_code: "US", latitude:  36.1627, longitude:  -86.7816 },
    City { name: "Detroit",        state: "Michigan",        country: "United States", country_code: "US", latitude:  42.3314, longitude:  -83.0458 },
    City { name: "Oklahoma City",  state: "Oklahoma",        country: "United States", country_code: "US", latitude:  35.4676, longitude:  -97.5164 },
    City { name: "Portland",       state: "Oregon",          country: "United States", country_code: "US", latitude:  45.5152, longitude: -122.6784 },
    City { name: "Las Vegas",      state: "Nevada",          country: "United States", country_code: "US", latitude:  36.1699, longitude: -115.1398 },
    City { name: "Memphis",        state: "Tennessee",       country: "United States", country_code: "US", latitude:  35.1495, longitude:  -90.0490 },
    City { name: "Louisville",     state: "Kentucky",        country: "United States", country_code: "US", latitude:  38.2527, longitude:  -85.7585 },
    City { name: "Baltimore",      state: "Maryland",        country: "United States", country_code: "US", latitude:  39.2904, longitude:  -76.6122 },
    City { name: "Milwaukee",      state: "Wisconsin",       country: "United States", country_code: "US", latitude:  43.0389, longitude:  -87.9065 },
    City { name: "Albuquerque",    state: "New Mexico",      country: "United States", country_code: "US", latitude:  35.0844, longitude: -106.6504 },
    City { name: "Tucson",         state: "Arizona",         country: "United States", country_code: "US", latitude:  32.2226, longitude: -110.9747 },
    City { name: "Fresno",         state: "California",      country: "United States", country_code: "US", latitude:  36.7378, longitude: -119.7871 },
    City { name: "Sacramento",     state: "California",      country: "United States", country_code: "US", latitude:  38.5816, longitude: -121.4944 },
    City { name: "Mesa",           state: "Arizona",         country: "United States", country_code: "US", latitude:  33.4152, longitude: -111.8315 },
    City { name: "Kansas City",    state: "Missouri",        country: "United States", country_code: "US", latitude:  39.0997, longitude:  -94.5786 },
    City { name: "Atlanta",        state: "Georgia",         country: "United States", country_code: "US", latitude:  33.7490, longitude:  -84.3880 },
    City { name: "Omaha",          state: "Nebraska",        country: "United States", country_code: "US", latitude:  41.2565, longitude:  -95.9345 },
    City { name: "Raleigh",        state: "North Carolina",  country: "United States", country_code: "US", latitude:  35.7796, longitude:  -78.6382 },
    City { name: "Miami",          state: "Florida",         country: "United States", country_code: "US", latitude:  25.7617, longitude:  -80.1918 },
    City { name: "Long Beach",     state: "California",      country: "United States", country_code: "US", latitude:  33.7701, longitude: -118.1937 },
    City { name: "Oakland",        state: "California",      country: "United States", country_code: "US", latitude:  37.8044, longitude: -122.2711 },
    City { name: "Minneapolis",    state: "Minnesota",       country: "United States", country_code: "US", latitude:  44.9778, longitude:  -93.2650 },
    City { name: "Tulsa",          state: "Oklahoma",        country: "United States", country_code: "US", latitude:  36.1540, longitude:  -95.9928 },
    City { name: "Arlington",      state: "Texas",           country: "United States", country_code: "US", latitude:  32.7357, longitude:  -97.1081 },
    City { name: "New Orleans",    state: "Louisiana",       country: "United States", country_code: "US", latitude:  29.9511, longitude:  -90.0715 },
    City { name: "Wichita",        state: "Kansas",          country: "United States", country_code: "US", latitude:  37.6872, longitude:  -97.3301 },
    City { name: "Cleveland",      state: "Ohio",            country: "United States", country_code: "US", latitude:  41.4993, longitude:  -81.6944 },
    City { name: "Tampa",          state: "Florida",         country: "United States", country_code: "US", latitude:  27.9506, longitude:  -82.4572 },
    City { name: "Bakersfield",    state: "California",      country: "United States", country_code: "US", latitude:  35.3733, longitude: -119.0187 },
    City { name: "Aurora",         state: "Colorado",        country: "United States", country_code: "US", latitude:  39.7294, longitude: -104.8319 },
    City { name: "Anaheim",        state: "California",      country: "United States", country_code: "US", latitude:  33.8366, longitude: -117.9143 },
    City { name: "Honolulu",       state: "Hawaii",          country: "United States", country_code: "US", latitude:  21.3069, longitude: -157.8583 },
    City { name: "Santa Ana",      state: "California",      country: "United States", country_code: "US", latitude:  33.7455, longitude: -117.8678 },
    City { name: "Riverside",      state: "California",      country: "United States", country_code: "US", latitude:  33.9806, longitude: -117.3755 },
    City { name: "Corpus Christi", state: "Texas",           country: "United States", country_code: "US", latitude:  27.8006, longitude:  -97.3964 },
    City { name: "Lexington",      state: "Kentucky",        country: "United States", country_code: "US", latitude:  38.0406, longitude:  -84.5037 },
    City { name: "Stockton",       state: "California",      country: "United States", country_code: "US", latitude:  37.9577, longitude: -121.2908 },
    City { name: "Henderson",      state: "Nevada",          country: "United States", country_code: "US", latitude:  36.0395, longitude: -114.9817 },
    City { name: "Saint Paul",     state: "Minnesota",       country: "United States", country_code: "US", latitude:  44.9537, longitude:  -93.0900 },
    City { name: "St. Louis",      state: "Missouri",        country: "United States", country_code: "US", latitude:  38.6270, longitude:  -90.1994 },
    City { name: "Cincinnati",     state: "Ohio",            country: "United States", country_code: "US", latitude:  39.1031, longitude:  -84.5120 },
    City { name: "Pittsburgh",     state: "Pennsylvania",    country: "United States", country_code: "US", latitude:  40.4406, longitude:  -79.9959 },
    City { name: "Anchorage",      state: "Alaska",          country: "United States", country_code: "US", latitude:  61.2181, longitude: -149.9003 },
    City { name: "Salt Lake City", state: "Utah",            country: "United States", country_code: "US", latitude:  40.7608, longitude: -111.8910 },
    City { name: "Madison",        state: "Wisconsin",       country: "United States", country_code: "US", latitude:  43.0731, longitude:  -89.4012 },
    City { name: "Boise",          state: "Idaho",           country: "United States", country_code: "US", latitude:  43.6150, longitude: -116.2023 },
    City { name: "Spokane",        state: "Washington",      country: "United States", country_code: "US", latitude:  47.6588, longitude: -117.4260 },
    City { name: "Buffalo",        state: "New York",        country: "United States", country_code: "US", latitude:  42.8864, longitude:  -78.8784 },
    City { name: "Irvine",         state: "California",      country: "United States", country_code: "US", latitude:  33.6846, longitude: -117.8265 },
    City { name: "Berkeley",       state: "California",      country: "United States", country_code: "US", latitude:  37.8716, longitude: -122.2727 },
    City { name: "Palo Alto",      state: "California",      country: "United States", country_code: "US", latitude:  37.4419, longitude: -122.1430 },
    City { name: "Pasadena",       state: "California",      country: "United States", country_code: "US", latitude:  34.1478, longitude: -118.1445 },
    City { name: "Santa Barbara",  state: "California",      country: "United States", country_code: "US", latitude:  34.4208, longitude: -119.6982 },
    City { name: "Cambridge",      state: "Massachusetts",   country: "United States", country_code: "US", latitude:  42.3736, longitude:  -71.1097 },
    // ── Canada ────────────────────────────────────────────────────────
    City { name: "Toronto",        state: "Ontario",         country: "Canada",         country_code: "CA", latitude:  43.6532, longitude:  -79.3832 },
    City { name: "Montreal",       state: "Quebec",          country: "Canada",         country_code: "CA", latitude:  45.5017, longitude:  -73.5673 },
    City { name: "Vancouver",      state: "British Columbia",country: "Canada",         country_code: "CA", latitude:  49.2827, longitude: -123.1207 },
    City { name: "Calgary",        state: "Alberta",         country: "Canada",         country_code: "CA", latitude:  51.0447, longitude: -114.0719 },
    City { name: "Ottawa",         state: "Ontario",         country: "Canada",         country_code: "CA", latitude:  45.4215, longitude:  -75.6972 },
    City { name: "Edmonton",       state: "Alberta",         country: "Canada",         country_code: "CA", latitude:  53.5461, longitude: -113.4938 },
    City { name: "Winnipeg",       state: "Manitoba",        country: "Canada",         country_code: "CA", latitude:  49.8951, longitude:  -97.1384 },
    // ── Mexico + Central + South America ───────────────────────────────
    City { name: "Mexico City",    state: "Mexico City",     country: "Mexico",         country_code: "MX", latitude:  19.4326, longitude:  -99.1332 },
    City { name: "Guadalajara",    state: "Jalisco",         country: "Mexico",         country_code: "MX", latitude:  20.6597, longitude: -103.3496 },
    City { name: "Monterrey",      state: "Nuevo León",      country: "Mexico",         country_code: "MX", latitude:  25.6866, longitude: -100.3161 },
    City { name: "São Paulo",      state: "São Paulo",       country: "Brazil",         country_code: "BR", latitude: -23.5505, longitude:  -46.6333 },
    City { name: "Rio de Janeiro", state: "Rio de Janeiro",  country: "Brazil",         country_code: "BR", latitude: -22.9068, longitude:  -43.1729 },
    City { name: "Buenos Aires",   state: "Buenos Aires",    country: "Argentina",      country_code: "AR", latitude: -34.6037, longitude:  -58.3816 },
    City { name: "Santiago",       state: "Santiago",        country: "Chile",          country_code: "CL", latitude: -33.4489, longitude:  -70.6693 },
    City { name: "Bogotá",         state: "Bogotá",          country: "Colombia",       country_code: "CO", latitude:   4.7110, longitude:  -74.0721 },
    City { name: "Lima",           state: "Lima",            country: "Peru",           country_code: "PE", latitude: -12.0464, longitude:  -77.0428 },
    City { name: "Caracas",        state: "Capital District",country: "Venezuela",      country_code: "VE", latitude:  10.4806, longitude:  -66.9036 },
    City { name: "Quito",          state: "Pichincha",       country: "Ecuador",        country_code: "EC", latitude:  -0.1807, longitude:  -78.4678 },
    City { name: "Havana",         state: "La Habana",       country: "Cuba",           country_code: "CU", latitude:  23.1136, longitude:  -82.3666 },
    // ── Europe ────────────────────────────────────────────────────────
    City { name: "London",         state: "England",         country: "United Kingdom", country_code: "GB", latitude:  51.5074, longitude:   -0.1278 },
    City { name: "Edinburgh",      state: "Scotland",        country: "United Kingdom", country_code: "GB", latitude:  55.9533, longitude:   -3.1883 },
    City { name: "Dublin",         state: "Leinster",        country: "Ireland",        country_code: "IE", latitude:  53.3498, longitude:   -6.2603 },
    City { name: "Paris",          state: "Île-de-France",   country: "France",         country_code: "FR", latitude:  48.8566, longitude:    2.3522 },
    City { name: "Madrid",         state: "Madrid",          country: "Spain",          country_code: "ES", latitude:  40.4168, longitude:   -3.7038 },
    City { name: "Barcelona",      state: "Catalonia",       country: "Spain",          country_code: "ES", latitude:  41.3851, longitude:    2.1734 },
    City { name: "Lisbon",         state: "Lisbon",          country: "Portugal",       country_code: "PT", latitude:  38.7223, longitude:   -9.1393 },
    City { name: "Berlin",         state: "Berlin",          country: "Germany",        country_code: "DE", latitude:  52.5200, longitude:   13.4050 },
    City { name: "Munich",         state: "Bavaria",         country: "Germany",        country_code: "DE", latitude:  48.1351, longitude:   11.5820 },
    City { name: "Hamburg",        state: "Hamburg",         country: "Germany",        country_code: "DE", latitude:  53.5511, longitude:    9.9937 },
    City { name: "Amsterdam",      state: "North Holland",   country: "Netherlands",    country_code: "NL", latitude:  52.3676, longitude:    4.9041 },
    City { name: "Brussels",       state: "Brussels",        country: "Belgium",        country_code: "BE", latitude:  50.8503, longitude:    4.3517 },
    City { name: "Zurich",         state: "Zurich",          country: "Switzerland",    country_code: "CH", latitude:  47.3769, longitude:    8.5417 },
    City { name: "Geneva",         state: "Geneva",          country: "Switzerland",    country_code: "CH", latitude:  46.2044, longitude:    6.1432 },
    City { name: "Vienna",         state: "Vienna",          country: "Austria",        country_code: "AT", latitude:  48.2082, longitude:   16.3738 },
    City { name: "Prague",         state: "Prague",          country: "Czechia",        country_code: "CZ", latitude:  50.0755, longitude:   14.4378 },
    City { name: "Warsaw",         state: "Masovian",        country: "Poland",         country_code: "PL", latitude:  52.2297, longitude:   21.0122 },
    City { name: "Budapest",       state: "Budapest",        country: "Hungary",        country_code: "HU", latitude:  47.4979, longitude:   19.0402 },
    City { name: "Rome",           state: "Lazio",           country: "Italy",          country_code: "IT", latitude:  41.9028, longitude:   12.4964 },
    City { name: "Milan",          state: "Lombardy",        country: "Italy",          country_code: "IT", latitude:  45.4642, longitude:    9.1900 },
    City { name: "Athens",         state: "Attica",          country: "Greece",         country_code: "GR", latitude:  37.9838, longitude:   23.7275 },
    City { name: "Istanbul",       state: "Istanbul",        country: "Turkey",         country_code: "TR", latitude:  41.0082, longitude:   28.9784 },
    City { name: "Stockholm",      state: "Stockholm",       country: "Sweden",         country_code: "SE", latitude:  59.3293, longitude:   18.0686 },
    City { name: "Oslo",           state: "Oslo",            country: "Norway",         country_code: "NO", latitude:  59.9139, longitude:   10.7522 },
    City { name: "Copenhagen",     state: "Capital Region",  country: "Denmark",        country_code: "DK", latitude:  55.6761, longitude:   12.5683 },
    City { name: "Helsinki",       state: "Uusimaa",         country: "Finland",        country_code: "FI", latitude:  60.1699, longitude:   24.9384 },
    City { name: "Reykjavik",      state: "Capital Region",  country: "Iceland",        country_code: "IS", latitude:  64.1466, longitude:  -21.9426 },
    City { name: "Moscow",         state: "Moscow",          country: "Russia",         country_code: "RU", latitude:  55.7558, longitude:   37.6173 },
    City { name: "Saint Petersburg", state: "Saint Petersburg", country: "Russia",      country_code: "RU", latitude:  59.9311, longitude:   30.3609 },
    City { name: "Kyiv",           state: "Kyiv",            country: "Ukraine",        country_code: "UA", latitude:  50.4501, longitude:   30.5234 },
    // ── Middle East + Africa ──────────────────────────────────────────
    City { name: "Cairo",          state: "Cairo",           country: "Egypt",          country_code: "EG", latitude:  30.0444, longitude:   31.2357 },
    City { name: "Casablanca",     state: "Casablanca-Settat", country: "Morocco",      country_code: "MA", latitude:  33.5731, longitude:   -7.5898 },
    City { name: "Lagos",          state: "Lagos",           country: "Nigeria",        country_code: "NG", latitude:   6.5244, longitude:    3.3792 },
    City { name: "Nairobi",        state: "Nairobi",         country: "Kenya",          country_code: "KE", latitude:  -1.2921, longitude:   36.8219 },
    City { name: "Addis Ababa",    state: "Addis Ababa",     country: "Ethiopia",       country_code: "ET", latitude:   9.0320, longitude:   38.7469 },
    City { name: "Johannesburg",   state: "Gauteng",         country: "South Africa",   country_code: "ZA", latitude: -26.2041, longitude:   28.0473 },
    City { name: "Cape Town",      state: "Western Cape",    country: "South Africa",   country_code: "ZA", latitude: -33.9249, longitude:   18.4241 },
    City { name: "Tel Aviv",       state: "Tel Aviv",        country: "Israel",         country_code: "IL", latitude:  32.0853, longitude:   34.7818 },
    City { name: "Jerusalem",      state: "Jerusalem",       country: "Israel",         country_code: "IL", latitude:  31.7683, longitude:   35.2137 },
    City { name: "Riyadh",         state: "Riyadh",          country: "Saudi Arabia",   country_code: "SA", latitude:  24.7136, longitude:   46.6753 },
    City { name: "Dubai",          state: "Dubai",           country: "United Arab Emirates", country_code: "AE", latitude:  25.2048, longitude:  55.2708 },
    City { name: "Tehran",         state: "Tehran",          country: "Iran",           country_code: "IR", latitude:  35.6892, longitude:   51.3890 },
    // ── Asia + Oceania ────────────────────────────────────────────────
    City { name: "Mumbai",         state: "Maharashtra",     country: "India",          country_code: "IN", latitude:  19.0760, longitude:   72.8777 },
    City { name: "Delhi",          state: "Delhi",           country: "India",          country_code: "IN", latitude:  28.7041, longitude:   77.1025 },
    City { name: "Bengaluru",      state: "Karnataka",       country: "India",          country_code: "IN", latitude:  12.9716, longitude:   77.5946 },
    City { name: "Chennai",        state: "Tamil Nadu",      country: "India",          country_code: "IN", latitude:  13.0827, longitude:   80.2707 },
    City { name: "Kolkata",        state: "West Bengal",     country: "India",          country_code: "IN", latitude:  22.5726, longitude:   88.3639 },
    City { name: "Hyderabad",      state: "Telangana",       country: "India",          country_code: "IN", latitude:  17.3850, longitude:   78.4867 },
    City { name: "Karachi",        state: "Sindh",           country: "Pakistan",       country_code: "PK", latitude:  24.8607, longitude:   67.0011 },
    City { name: "Dhaka",          state: "Dhaka",           country: "Bangladesh",     country_code: "BD", latitude:  23.8103, longitude:   90.4125 },
    City { name: "Bangkok",        state: "Bangkok",         country: "Thailand",       country_code: "TH", latitude:  13.7563, longitude:  100.5018 },
    City { name: "Singapore",      state: "Central Region",  country: "Singapore",      country_code: "SG", latitude:   1.3521, longitude:  103.8198 },
    City { name: "Jakarta",        state: "Jakarta",         country: "Indonesia",      country_code: "ID", latitude:  -6.2088, longitude:  106.8456 },
    City { name: "Manila",         state: "Metro Manila",    country: "Philippines",    country_code: "PH", latitude:  14.5995, longitude:  120.9842 },
    City { name: "Kuala Lumpur",   state: "Federal Territory of Kuala Lumpur", country: "Malaysia", country_code: "MY", latitude:   3.1390, longitude: 101.6869 },
    City { name: "Ho Chi Minh City", state: "Ho Chi Minh",   country: "Vietnam",        country_code: "VN", latitude:  10.8231, longitude:  106.6297 },
    City { name: "Hong Kong",      state: "Hong Kong",       country: "China",          country_code: "HK", latitude:  22.3193, longitude:  114.1694 },
    City { name: "Beijing",        state: "Beijing",         country: "China",          country_code: "CN", latitude:  39.9042, longitude:  116.4074 },
    City { name: "Shanghai",       state: "Shanghai",        country: "China",          country_code: "CN", latitude:  31.2304, longitude:  121.4737 },
    City { name: "Guangzhou",      state: "Guangdong",       country: "China",          country_code: "CN", latitude:  23.1291, longitude:  113.2644 },
    City { name: "Shenzhen",       state: "Guangdong",       country: "China",          country_code: "CN", latitude:  22.5431, longitude:  114.0579 },
    City { name: "Taipei",         state: "Taipei",          country: "Taiwan",         country_code: "TW", latitude:  25.0330, longitude:  121.5654 },
    City { name: "Seoul",          state: "Seoul",           country: "South Korea",    country_code: "KR", latitude:  37.5665, longitude:  126.9780 },
    City { name: "Tokyo",          state: "Tokyo",           country: "Japan",          country_code: "JP", latitude:  35.6762, longitude:  139.6503 },
    City { name: "Osaka",          state: "Osaka",           country: "Japan",          country_code: "JP", latitude:  34.6937, longitude:  135.5023 },
    City { name: "Kyoto",          state: "Kyoto",           country: "Japan",          country_code: "JP", latitude:  35.0116, longitude:  135.7681 },
    City { name: "Sydney",         state: "New South Wales", country: "Australia",      country_code: "AU", latitude: -33.8688, longitude:  151.2093 },
    City { name: "Melbourne",      state: "Victoria",        country: "Australia",      country_code: "AU", latitude: -37.8136, longitude:  144.9631 },
    City { name: "Brisbane",       state: "Queensland",      country: "Australia",      country_code: "AU", latitude: -27.4698, longitude:  153.0251 },
    City { name: "Perth",          state: "Western Australia", country: "Australia",    country_code: "AU", latitude: -31.9523, longitude:  115.8613 },
    City { name: "Adelaide",       state: "South Australia", country: "Australia",      country_code: "AU", latitude: -34.9285, longitude:  138.6007 },
    City { name: "Auckland",       state: "Auckland",        country: "New Zealand",    country_code: "NZ", latitude: -36.8485, longitude:  174.7633 },
    City { name: "Wellington",     state: "Wellington",      country: "New Zealand",    country_code: "NZ", latitude: -41.2865, longitude:  174.7762 },
];

/// Perform a search on the city database.
///
/// Mirrors the FTS5-then-Soundex fallback matrix described in section 3.1 of
/// `implementation.md`: prefix/contains match first; if that returns nothing,
/// fall through to phonetic lookup keyed by [`soundex`].
pub fn search_cities(query: &str) -> Vec<City> {
    let clean_query = query.trim().to_lowercase();
    if clean_query.is_empty() {
        return BUILTIN_CITIES.to_vec();
    }

    let prefix_results: Vec<City> = BUILTIN_CITIES
        .iter()
        .copied()
        .filter(|city| {
            let name = city.name.to_lowercase();
            name.starts_with(&clean_query)
                || name.contains(&clean_query)
                || city.state.to_lowercase().starts_with(&clean_query)
                || city.country.to_lowercase().starts_with(&clean_query)
        })
        .collect();

    if !prefix_results.is_empty() {
        return prefix_results;
    }

    let query_soundex = soundex(&clean_query);
    BUILTIN_CITIES
        .iter()
        .copied()
        .filter(|city| soundex(city.name) == query_soundex)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soundex_algorithm() {
        // Spot-check standard Soundex outputs.
        assert_eq!(soundex("Denver"), "D516");
        assert_eq!(soundex("London"), "L535");
        assert_eq!(soundex("Paris"), "P620");
        // Classic textbook equivalence (Wikipedia Soundex example).
        assert_eq!(soundex("Robert"), soundex("Rupert"));
        // H and W skip adjacency, so "Honeyman" / "Honeymun" stay equal.
        assert_eq!(soundex("Honeyman"), soundex("Honeymun"));
    }

    #[test]
    fn test_city_search_prefix() {
        // Exact prefix.
        let res = search_cities("Denv");
        assert!(
            res.iter().any(|c| c.name == "Denver"),
            "Denver should be findable by 'Denv' prefix"
        );

        // Case insensitive prefix.
        let res2 = search_cities("tokyo");
        assert!(res2.iter().any(|c| c.name == "Tokyo"));
    }

    /// User reported "I typed irvine and search did not find anything"
    /// because the first cut of the catalog was only 18 cities and
    /// did not include Irvine. Catalog now has ~150 entries.
    #[test]
    fn test_irvine_findable() {
        let res = search_cities("Irvine");
        assert!(
            res.iter().any(|c| c.name == "Irvine"),
            "Irvine must be in the built-in catalog"
        );
        let r2 = search_cities("irvine");
        assert!(r2.iter().any(|c| c.name == "Irvine"), "case-insensitive too");
    }

    #[test]
    fn test_city_search_phonetic_fallback() {
        // "Tokio" misspells "Tokyo" — same Soundex code so the phonetic
        // fallback should find it once the prefix branch returns nothing.
        let res = search_cities("Tokio");
        assert!(
            res.iter().any(|c| c.name == "Tokyo"),
            "Soundex fallback should resolve Tokio → Tokyo, got {:?}",
            res.iter().map(|c| c.name).collect::<Vec<_>>()
        );
    }
}
