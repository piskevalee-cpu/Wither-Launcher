// src-tauri/src/steam/store_api.rs
// Module 11 — Steam Store API Client

use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const STORE_BASE_URL: &str = "https://store.steampowered.com/api";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturedItem {
    pub id: u32,
    pub name: String,
    pub discounted: bool,
    pub discount_percent: Option<i32>,
    pub original_price: Option<u64>,
    pub final_price: Option<u64>,
    pub currency: Option<String>,
    pub header_image: String,
    pub large_capsule_image: Option<String>,
}

// Helper to parse featured items from various API response formats
fn parse_featured_items(value: &Value) -> Vec<FeaturedItem> {
    let mut items = Vec::new();
    
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Ok(featured_item) = serde_json::from_value::<FeaturedItem>(item.clone()) {
                items.push(featured_item);
            }
        }
    }
    
    items
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDetails {
    pub steam_appid: u32,
    pub name: String,
    pub short_description: Option<String>,
    pub header_image: Option<String>,
    pub developers: Option<Vec<String>>,
    pub publishers: Option<Vec<String>>,
    pub is_free: bool,
    pub price_overview: Option<PriceOverview>,
    pub genres: Option<Vec<Genre>>,
    pub release_date: Option<ReleaseDate>,
    pub screenshots: Option<Vec<Screenshot>>,
    pub movies: Option<Vec<Movie>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceOverview {
    #[serde(default)]
    pub initial_formatted: String,
    #[serde(default)]
    pub final_formatted: String,
    #[serde(default)]
    pub discount_percent: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genre {
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseDate {
    #[serde(default)]
    pub coming_soon: bool,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub path_thumbnail: String,
    pub path_full: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub mp4: Option<MovieMp4>,
    pub webm: Option<MovieWebm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieMp4 {
    #[serde(rename = "480")]
    pub quality_480: Option<String>,
    pub max: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieWebm {
    #[serde(rename = "480")]
    pub quality_480: Option<String>,
    pub max: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    pub name: String,
    pub id: u32,
    pub tiny_image: Option<String>,
    pub price: Option<SearchPrice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPrice {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub initial: i32,
    #[serde(default, rename = "final")]
    pub final_val: i32,
    #[serde(default)]
    pub discount_percent: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultsRaw {
    pub total: u32,
    pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItemFrontend {
    pub name: String,
    pub id: u32,
    pub logo: Option<String>,
    pub price: Option<String>,
    pub sale_price: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub total_count: u32,
    pub items: Vec<SearchItemFrontend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseItem {
    pub name: String,
    pub id: String,
    pub logo: Option<String>,
    pub price: Option<String>,
    pub sale_price: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseResults {
    pub items: Vec<BrowseItem>,
}

pub struct SteamStoreApi {
    client: Client,
    cc: String,  // Country code
    l: String,   // Language
}

impl SteamStoreApi {
    pub fn new(cc: &str, l: &str) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        );
        headers.insert(
            reqwest::header::REFERER,
            reqwest::header::HeaderValue::from_static("https://store.steampowered.com/")
        );
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json, text/plain, */*")
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("en-US,en;q=0.9")
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            cc: cc.to_string(),
            l: l.to_string(),
        }
    }

    /// Get featured games (homepage)
    pub async fn get_featured(&self) -> Result<Vec<FeaturedItem>, String> {
        let url = format!(
            "{}/featured/?cc={}&l={}",
            STORE_BASE_URL, self.cc, self.l
        );
        
        info!("Fetching featured games from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch featured: {}", e))?;
        
        let data: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse featured response: {}", e))?;
        
        let mut all_items = Vec::new();
        
        // Parse large_capsules
        if let Some(large_capsules) = data.get("large_capsules") {
            all_items.extend(parse_featured_items(large_capsules));
        }
        
        // Parse featured_win
        if let Some(featured_win) = data.get("featured_win") {
            all_items.extend(parse_featured_items(featured_win));
        }
        
        // Parse specials.items
        if let Some(specials) = data.get("specials") {
            if let Some(items) = specials.get("items") {
                all_items.extend(parse_featured_items(items));
            }
        }
        
        // Remove duplicates by id
        all_items.sort_by(|a, b| a.id.cmp(&b.id));
        all_items.dedup_by(|a, b| a.id == b.id);
        
        Ok(all_items)
    }

    /// Get featured categories (top sellers, new releases, etc.)
    pub async fn get_featured_categories(&self) -> Result<serde_json::Value, String> {
        let url = format!(
            "{}/featuredcategories/?cc={}&l={}",
            STORE_BASE_URL, self.cc, self.l
        );
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch categories: {}", e))?;
        
        let data: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse categories response: {}", e))?;
        
        Ok(data)
    }

    /// Get app details for a single game
    pub async fn get_app_details(&self, app_id: u32) -> Result<AppDetails, String> {
        let url = format!(
            "{}/appdetails/?appids={}&cc={}&l={}&filters=basic,price_overview,genres,developers,release_date,screenshots,movies",
            STORE_BASE_URL, app_id, self.cc, self.l
        );
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch app details: {}", e))?;
        
        // Response is { "appid": { "success": true, "data": {...} } }
        let mut data: HashMap<String, serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse app details response: {}", e))?;
        
        let app_data = data
            .remove(&app_id.to_string())
            .ok_or_else(|| format!("No data for app {}", app_id))?;
        
        let success = app_data
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        if !success {
            return Err(format!("App {} not found or unavailable", app_id));
        }
        
        let details: AppDetails = serde_json::from_value(
            app_data.get("data").cloned().unwrap_or_default()
        ).map_err(|e| format!("Failed to parse app details: {}", e))?;
        
        Ok(details)
    }

    /// Search games by query
    pub async fn search(&self, query: &str, page: u32, count: u32) -> Result<SearchResults, String> {
        let start = page * count;
        let url = format!(
            "{}/storesearch/?term={}&cc={}&l={}&count={}&start={}",
            STORE_BASE_URL, 
            urlencoding::encode(query),
            self.cc, self.l, count, start
        );
        
        info!("Search URL: {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Search failed: {}", e))?;
        
        let status = response.status();
        let body = response.text().await.map_err(|e| format!("Failed to get body: {}", e))?;
        
        if !status.is_success() {
            return Err(format!("Search failed with status {}: {}", status, body.chars().take(200).collect::<String>()));
        }

        let raw: SearchResultsRaw = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse search results (Body: {}): {}", body.chars().take(100).collect::<String>(), e))?;
        
        // Convert to frontend format
        let items = raw.items.into_iter().map(|item| {
            let price_str = item.price.as_ref().map(|p| {
                if p.final_val == 0 {
                    "Free".to_string()
                } else {
                    format!("{:.2} {}", (p.final_val as f64) / 100.0, p.currency)
                }
            });
            
            let sale_price = item.price.as_ref().and_then(|p| {
                if p.discount_percent > 0 {
                    Some(format!("{:.2} {}", (p.initial as f64) / 100.0, p.currency))
                } else {
                    None
                }
            });

            SearchItemFrontend {
                name: item.name,
                id: item.id,
                logo: item.tiny_image,
                price: price_str,
                sale_price,
            }
        }).collect();

        Ok(SearchResults {
            total_count: raw.total,
            items,
        })
    }

    /// Browse games by category filter
    pub async fn browse(&self, filter: &str, page: u32, count: u32) -> Result<SearchResults, String> {
        let start = page * count;
        let url = format!(
            "https://store.steampowered.com/search/results/?filter={}&json=1&cc={}&l={}&count={}&start={}",
            filter, self.cc, self.l, count, start
        );
        
        info!("Browse URL: {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Browse failed: {}", e))?;
        
        let status = response.status();
        let body = response.text().await.map_err(|e| format!("Failed to get body: {}", e))?;
        
        if !status.is_success() {
            return Err(format!("Browse failed with status {}: {}", status, body.chars().take(200).collect::<String>()));
        }

        let raw: BrowseResults = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse browse results (Body: {}): {}", body.chars().take(100).collect::<String>(), e))?;
        
        // Convert to SearchResults format
        let items = raw.items.into_iter().map(|item| {
            let id = item.id.parse::<u32>().unwrap_or(0);
            SearchItemFrontend {
                name: item.name,
                id,
                logo: item.logo,
                price: item.price,
                sale_price: item.sale_price,
            }
        }).collect::<Vec<_>>();

        let total_count = items.len() as u32;

        Ok(SearchResults {
            total_count,
            items,
        })
    }
}
