// src/lib/stores/storeStore.ts
// Module 11 — Steam Store State Management

import { writable, derived } from 'svelte/store'
import { invoke } from '@tauri-apps/api/core'

// Types matching Rust backend
export interface FeaturedItem {
  id:         number
  name:       string
  discounted: boolean
  discount_percent: number | null
  original_price:   number | null
  final_price:      number | null
  currency:         string | null
  header_image:     string
}

export interface AppDetails {
  steam_appid:      number
  name:             string
  short_description: string | null
  header_image:     string | null
  developers:       string[] | null
  is_free:          boolean
  price_overview:   PriceOverview | null
  genres:           Genre[] | null
  release_date:     ReleaseDate | null
  screenshots:      Screenshot[] | null
}

export interface PriceOverview {
  initial_formatted: string
  final_formatted:   string
  discount_percent:  number
}

export interface Genre {
  description: string
}

export interface ReleaseDate {
  coming_soon: boolean
  date:        string
}

export interface Screenshot {
  path_thumbnail: string
  path_full:      string
}

export interface SearchItem {
  name:       string
  id:         number
  logo:       string | null
  price:      string | null
  sale_price: string | null
}

// Store state
export const featured = writable<FeaturedItem[]>([])
export const topSellers = writable<SearchItem[]>([])
export const newReleases = writable<SearchItem[]>([])
export const specials = writable<SearchItem[]>([])
export const currentAppDetails = writable<AppDetails | null>(null)
export const searchResults = writable<SearchItem[]>([])
export const searchQuery = writable<string>('')
export const isLoading = writable<boolean>(false)
export const error = writable<string | null>(null)

// Load featured games (home page)
export async function loadFeatured(): Promise<void> {
  isLoading.set(true)
  error.set(null)
  try {
    const result = await invoke<FeaturedItem[]>('store_get_featured')
    featured.set(result)
  } catch (e) {
    error.set(String(e))
  } finally {
    isLoading.set(false)
  }
}

// Load category data (top sellers, new releases, specials)
export async function loadCategories(): Promise<void> {
  isLoading.set(true)
  error.set(null)
  try {
    const result = await invoke<any>('store_get_categories')
    
    // Parse top_sellers - can be array directly or { items: [...] }
    if (result.top_sellers) {
      const items = Array.isArray(result.top_sellers) 
        ? result.top_sellers 
        : result.top_sellers.items || [];
      topSellers.set(items)
    }
    
    // Parse new_releases
    if (result.new_releases) {
      const items = Array.isArray(result.new_releases) 
        ? result.new_releases 
        : result.new_releases.items || [];
      newReleases.set(items)
    }
    
    // Parse specials
    if (result.specials) {
      const items = Array.isArray(result.specials) 
        ? result.specials 
        : result.specials.items || [];
      specials.set(items)
    }
  } catch (e) {
    error.set(String(e))
  } finally {
    isLoading.set(false)
  }
}

// Load app details for a single game
export async function loadAppDetails(appId: number): Promise<void> {
  isLoading.set(true)
  error.set(null)
  try {
    const result = await invoke<AppDetails>('store_get_app', { appId })
    currentAppDetails.set(result)
  } catch (e) {
    error.set(String(e))
  } finally {
    isLoading.set(false)
  }
}

// Search games
export async function searchGames(query: string, page = 0): Promise<void> {
  if (!query.trim()) {
    searchResults.set([])
    return
  }
  
  isLoading.set(true)
  error.set(null)
  try {
    const result = await invoke<SearchItem[]>('store_search', { query, page })
    searchResults.set(result)
  } catch (e) {
    error.set(String(e))
  } finally {
    isLoading.set(false)
  }
}

// Browse by category filter
export async function browseGames(filter: string, page = 0): Promise<SearchItem[]> {
  isLoading.set(true)
  error.set(null)
  try {
    const result = await invoke<SearchItem[]>('store_browse', { filter, page })
    return result
  } catch (e) {
    error.set(String(e))
    return []
  } finally {
    isLoading.set(false)
  }
}

// Open game page in Steam Store (browser)
export async function openInSteamStore(appId: number): Promise<void> {
  const url = `https://store.steampowered.com/app/${appId}`
  await invoke('open_url', { url })
}

// Check if game is already in library
export function isInLibrary(appId: number, library: any[]): boolean {
  return library.some(g => g.steam_app_id === appId)
}
