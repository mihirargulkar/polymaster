/// Market category system with keyword matching for Polymarket
/// and series ticker mapping for Kalshi.

use std::collections::HashMap;

pub struct CategoryRegistry {
    /// category:subcategory -> list of keywords for matching market titles
    keywords: HashMap<String, Vec<&'static str>>,
}

impl CategoryRegistry {
    pub fn new() -> Self {
        let mut keywords = HashMap::new();

        // Sports
        keywords.insert("sports:nba".into(), vec!["NBA", "basketball", "Lakers", "Celtics", "Warriors", "Bucks", "Thunder", "76ers", "Nuggets", "Knicks", "Heat", "Nets", "Suns", "Mavericks", "Clippers", "Cavaliers", "Timberwolves", "Pacers", "Pelicans", "Kings", "Hawks", "Bulls", "Pistons", "Rockets", "Spurs", "Grizzlies", "Raptors", "Trail Blazers", "Jazz", "Wizards", "Hornets", "Magic"]);
        keywords.insert("sports:nfl".into(), vec!["NFL", "football", "Super Bowl", "Chiefs", "Eagles", "49ers", "Bills", "Cowboys", "Dolphins", "Ravens", "Lions", "Bengals", "Chargers", "Jets", "Packers", "Seahawks", "Rams", "Steelers", "Browns", "Vikings", "Jaguars", "Broncos", "Saints", "Buccaneers", "Cardinals", "Colts", "Falcons", "Panthers", "Bears", "Commanders", "Titans", "Raiders", "Texans", "Giants", "Patriots"]);
        keywords.insert("sports:mlb".into(), vec!["MLB", "baseball", "Yankees", "Dodgers", "Mets", "Braves", "Astros", "Phillies", "Red Sox", "Cubs", "Padres", "Rangers", "Mariners", "Twins", "Orioles", "Guardians", "Rays", "Brewers", "Cardinals", "Blue Jays", "Giants", "Reds", "Pirates", "Diamondbacks", "Royals", "Tigers", "White Sox", "Rockies", "Angels", "Athletics", "Nationals", "Marlins"]);
        keywords.insert("sports:nhl".into(), vec!["NHL", "hockey", "Bruins", "Panthers", "Oilers", "Rangers", "Hurricanes", "Stars", "Avalanche", "Golden Knights", "Maple Leafs", "Lightning", "Devils", "Islanders", "Penguins", "Canucks", "Jets", "Kings", "Wild", "Senators", "Capitals", "Flames", "Predators", "Kraken", "Blue Jackets", "Flyers", "Red Wings", "Sabres", "Ducks", "Coyotes", "Sharks", "Blackhawks"]);
        keywords.insert("sports:soccer".into(), vec!["soccer", "football", "FIFA", "World Cup", "Premier League", "Champions League", "La Liga", "Bundesliga", "Serie A", "MLS", "Arsenal", "Manchester", "Liverpool", "Chelsea", "Barcelona", "Real Madrid", "Bayern", "PSG", "Juventus", "Inter Milan"]);
        keywords.insert("sports:golf".into(), vec!["golf", "PGA", "Masters", "Open Championship", "US Open golf", "Ryder Cup", "birdie", "eagle"]);
        keywords.insert("sports:mma".into(), vec!["UFC", "MMA", "fight", "bout", "knockout", "submission", "Octagon", "Dana White"]);
        keywords.insert("sports:tennis".into(), vec!["tennis", "ATP", "WTA", "Grand Slam", "Wimbledon", "Roland Garros", "US Open tennis", "Australian Open", "match", "Mannarino", "Shelton", "Djokovic", "Sinner", "Alcaraz", "Swiatek"]);
        keywords.insert("sports:college_football".into(), vec!["college football", "NCAA football", "CFB", "College Football Playoff", "Heisman", "NCAAF", "Bowl Game"]);
        keywords.insert("sports:college_basketball".into(), vec!["college basketball", "NCAA basketball", "March Madness", "NCAAB", "Final Four"]);

        // Politics
        keywords.insert("politics:us_elections".into(), vec!["President", "presidential", "election", "electoral", "White House", "nominee", "primary", "caucus", "swing state", "ballot", "vote", "campaign", "running mate", "vice president"]);
        keywords.insert("politics:congress".into(), vec!["Congress", "Senate", "House", "bill", "legislation", "filibuster", "committee", "Speaker", "impeach", "confirmation"]);
        keywords.insert("politics:policy".into(), vec!["policy", "regulation", "executive order", "tariff", "sanctions", "mandate", "government shutdown", "debt ceiling"]);
        keywords.insert("politics:international".into(), vec!["NATO", "EU", "United Nations", "G7", "G20", "Brexit", "trade deal", "summit", "diplomatic"]);

        // Economics
        keywords.insert("economics:fed".into(), vec!["Fed", "interest rate", "FOMC", "Federal Reserve", "rate cut", "rate hike", "monetary policy", "Jerome Powell", "basis points", "taper"]);
        keywords.insert("economics:inflation".into(), vec!["inflation", "CPI", "consumer price", "deflation", "price index", "PCE"]);
        keywords.insert("economics:jobs".into(), vec!["jobs", "unemployment", "nonfarm payroll", "jobless claims", "labor market", "hiring", "layoffs", "employment"]);
        keywords.insert("economics:gdp".into(), vec!["GDP", "gross domestic product", "economic growth", "recession", "contraction", "expansion"]);
        keywords.insert("economics:recession".into(), vec!["recession", "downturn", "depression", "economic decline", "yield curve"]);

        // Crypto
        keywords.insert("crypto:bitcoin".into(), vec!["Bitcoin", "BTC", "bitcoin price", "satoshi", "halving", "mining BTC"]);
        keywords.insert("crypto:ethereum".into(), vec!["Ethereum", "ETH", "ether", "Vitalik", "EIP", "staking ETH"]);
        keywords.insert("crypto:altcoins".into(), vec!["Solana", "SOL", "XRP", "Ripple", "Cardano", "ADA", "Dogecoin", "DOGE", "Polkadot", "DOT", "Avalanche", "AVAX", "Chainlink", "LINK", "Polygon", "MATIC", "Litecoin", "LTC"]);
        keywords.insert("crypto:regulation".into(), vec!["SEC crypto", "crypto regulation", "crypto ban", "stablecoin", "CBDC", "crypto ETF", "Bitcoin ETF"]);

        // Finance
        keywords.insert("finance:sp500".into(), vec!["S&P 500", "SPX", "SPY", "S&P", "SP500"]);
        keywords.insert("finance:nasdaq".into(), vec!["NASDAQ", "QQQ", "Nasdaq", "tech stocks"]);
        keywords.insert("finance:commodities".into(), vec!["gold price", "oil price", "silver", "crude oil", "WTI", "Brent", "commodity"]);
        keywords.insert("finance:forex".into(), vec!["EUR/USD", "USD/JPY", "GBP/USD", "forex", "currency pair", "dollar index", "DXY"]);
        keywords.insert("finance:stocks".into(), vec!["TSLA", "Tesla", "AAPL", "Apple", "NVDA", "NVIDIA", "AMZN", "Amazon", "GOOGL", "Google", "META", "Microsoft", "MSFT"]);

        // Weather
        keywords.insert("weather:temperature".into(), vec!["temperature", "high temp", "low temp", "degrees", "heat", "cold", "record high", "record low", "Fahrenheit", "Celsius"]);
        keywords.insert("weather:storms".into(), vec!["hurricane", "storm", "tornado", "cyclone", "typhoon", "tropical", "flooding", "blizzard"]);
        keywords.insert("weather:disasters".into(), vec!["earthquake", "wildfire", "tsunami", "volcanic", "drought", "natural disaster"]);

        // Tech
        keywords.insert("tech:ai".into(), vec!["AI", "artificial intelligence", "GPT", "Claude", "machine learning", "LLM", "OpenAI", "Anthropic", "deep learning", "neural"]);
        keywords.insert("tech:launches".into(), vec!["iPhone", "launch", "release", "product announcement", "keynote", "WWDC", "I/O"]);
        keywords.insert("tech:company".into(), vec!["IPO", "acquisition", "merger", "layoffs tech", "valuation", "funding round"]);

        // Culture
        keywords.insert("culture:entertainment".into(), vec!["Oscar", "Academy Award", "Emmy", "Grammy", "Golden Globe", "BAFTA", "box office", "streaming", "Netflix", "Disney"]);
        keywords.insert("culture:social".into(), vec!["Twitter", "TikTok", "Instagram", "viral", "trending", "influencer", "YouTube"]);
        keywords.insert("culture:celebrity".into(), vec!["celebrity", "scandal", "divorce", "award show", "concert", "tour"]);

        // World Events
        keywords.insert("world:geopolitics".into(), vec!["geopolitics", "conflict", "war", "invasion", "ceasefire", "peace deal", "coup", "regime"]);
        keywords.insert("world:conflicts".into(), vec!["Ukraine", "Russia", "Gaza", "Israel", "Taiwan", "China", "Iran", "North Korea", "military"]);
        keywords.insert("world:treaties".into(), vec!["treaty", "agreement", "accord", "pact", "alliance", "trade agreement", "climate accord"]);

        // Health
        keywords.insert("health:pandemics".into(), vec!["pandemic", "COVID", "virus", "outbreak", "epidemic", "WHO", "vaccine", "variant"]);
        keywords.insert("health:fda".into(), vec!["FDA", "drug approval", "clinical trial", "pharmaceutical", "EUA", "therapy"]);
        keywords.insert("health:public".into(), vec!["public health", "mortality", "life expectancy", "obesity", "mental health", "opioid"]);

        Self { keywords }
    }

    /// Map Kalshi's native category names to our internal category keys
    fn native_to_internal(native_category: &str) -> Option<&'static str> {
        let lower = native_category.to_lowercase();
        match lower.as_str() {
            "sports" | "nba" | "nfl" | "mlb" | "nhl" | "soccer" | "golf" | "mma" | "tennis"
            | "college-football" | "college-basketball" | "ncaa" => Some("sports"),
            "politics" | "elections" | "us-elections" | "congress" => Some("politics"),
            "economics" | "economy" | "fed" | "inflation" | "jobs" => Some("economics"),
            "crypto" | "cryptocurrency" | "bitcoin" | "ethereum" => Some("crypto"),
            "finance" | "stocks" | "markets" | "indices" => Some("finance"),
            "weather" | "climate" | "temperature" => Some("weather"),
            "tech" | "technology" | "ai" => Some("tech"),
            "culture" | "entertainment" | "awards" => Some("culture"),
            "world" | "geopolitics" | "international" => Some("world"),
            "health" | "healthcare" | "fda" => Some("health"),
            _ => None,
        }
    }

    /// Check if a native Kalshi category matches the user's selection
    pub fn matches_native_category(&self, native_category: &str, selected: &[String]) -> bool {
        if selected.iter().any(|s| s == "all") {
            return true;
        }

        if let Some(internal) = Self::native_to_internal(native_category) {
            for sel in selected {
                if sel == internal || sel.starts_with(&format!("{}:", internal)) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a market title matches the user's selected categories
    /// Returns (category, subcategory) if matched, None if not in user's selection
    pub fn matches_selection(&self, market_title: &str, selected: &[String]) -> Option<(String, String)> {
        // "all" matches everything
        if selected.iter().any(|s| s == "all") {
            // Still try to categorize for labeling purposes
            return self.categorize(market_title).or(Some(("uncategorized".into(), "other".into())));
        }

        let title_lower = market_title.to_lowercase();

        for selection in selected {
            if selection.ends_with(":all") {
                // e.g. "sports:all" — match any sports subcategory
                let category = selection.trim_end_matches(":all");
                let prefix = format!("{}:", category);
                for (key, kw_list) in &self.keywords {
                    if key.starts_with(&prefix) {
                        for kw in kw_list {
                            if title_lower.contains(&kw.to_lowercase()) {
                                let parts: Vec<&str> = key.splitn(2, ':').collect();
                                return Some((parts[0].to_string(), parts.get(1).unwrap_or(&"all").to_string()));
                            }
                        }
                    }
                }
            } else if let Some(kw_list) = self.keywords.get(selection) {
                // Specific subcategory match
                for kw in kw_list {
                    if title_lower.contains(&kw.to_lowercase()) {
                        let parts: Vec<&str> = selection.splitn(2, ':').collect();
                        return Some((parts[0].to_string(), parts.get(1).unwrap_or(&"all").to_string()));
                    }
                }
            }
        }

        None
    }

    /// Categorize a market title (best-effort, returns first match)
    pub fn categorize(&self, market_title: &str) -> Option<(String, String)> {
        let title_lower = market_title.to_lowercase();

        for (key, kw_list) in &self.keywords {
            for kw in kw_list {
                if title_lower.contains(&kw.to_lowercase()) {
                    let parts: Vec<&str> = key.splitn(2, ':').collect();
                    return Some((parts[0].to_string(), parts.get(1).unwrap_or(&"all").to_string()));
                }
            }
        }

        None
    }

    /// Get all top-level categories
    pub fn all_categories() -> Vec<(&'static str, &'static str)> {
        vec![
            ("sports", "Sports (NBA, NFL, MLB, NHL, Soccer, Golf...)"),
            ("politics", "Politics (Elections, Congress, Policy...)"),
            ("crypto", "Crypto (Bitcoin, Ethereum, Altcoins...)"),
            ("economics", "Economics (Fed rates, Inflation, Jobs, GDP...)"),
            ("finance", "Finance (S&P 500, NASDAQ, Stocks...)"),
            ("weather", "Weather (Temperature, Storms...)"),
            ("tech", "Tech (AI, Product launches...)"),
            ("culture", "Culture (Entertainment, Awards...)"),
            ("world", "World Events (Geopolitics, Conflicts...)"),
            ("health", "Health (FDA, Pandemics...)"),
        ]
    }

    /// Get subcategories for a given top-level category
    pub fn subcategories(category: &str) -> Vec<(&'static str, &'static str)> {
        match category {
            "sports" => vec![
                ("sports:nba", "NBA"),
                ("sports:nfl", "NFL"),
                ("sports:mlb", "MLB"),
                ("sports:nhl", "NHL"),
                ("sports:soccer", "Soccer"),
                ("sports:golf", "Golf"),
                ("sports:mma", "MMA/UFC"),
                ("sports:tennis", "Tennis"),
                ("sports:college_football", "College Football"),
                ("sports:college_basketball", "College Basketball"),
            ],
            "politics" => vec![
                ("politics:us_elections", "US Elections"),
                ("politics:congress", "Congress/Legislation"),
                ("politics:policy", "Policy/Regulation"),
                ("politics:international", "International Politics"),
            ],
            "crypto" => vec![
                ("crypto:bitcoin", "Bitcoin"),
                ("crypto:ethereum", "Ethereum"),
                ("crypto:altcoins", "Altcoins"),
                ("crypto:regulation", "Regulation"),
            ],
            "economics" => vec![
                ("economics:fed", "Fed/Interest Rates"),
                ("economics:inflation", "Inflation/CPI"),
                ("economics:jobs", "Jobs/Unemployment"),
                ("economics:gdp", "GDP"),
                ("economics:recession", "Recession"),
            ],
            "finance" => vec![
                ("finance:sp500", "S&P 500"),
                ("finance:nasdaq", "NASDAQ"),
                ("finance:commodities", "Commodities"),
                ("finance:forex", "Forex"),
                ("finance:stocks", "Individual Stocks"),
            ],
            "weather" => vec![
                ("weather:temperature", "Temperature"),
                ("weather:storms", "Storms/Hurricanes"),
                ("weather:disasters", "Natural Disasters"),
            ],
            "tech" => vec![
                ("tech:ai", "AI/ML"),
                ("tech:launches", "Product Launches"),
                ("tech:company", "Company Events"),
            ],
            "culture" => vec![
                ("culture:entertainment", "Entertainment/Awards"),
                ("culture:social", "Social Media"),
                ("culture:celebrity", "Celebrity"),
            ],
            "world" => vec![
                ("world:geopolitics", "Geopolitics"),
                ("world:conflicts", "Conflicts"),
                ("world:treaties", "Treaties/Agreements"),
            ],
            "health" => vec![
                ("health:pandemics", "Pandemics"),
                ("health:fda", "FDA/Drug Approvals"),
                ("health:public", "Public Health"),
            ],
            _ => vec![],
        }
    }
}
