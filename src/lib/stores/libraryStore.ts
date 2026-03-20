// src/lib/stores/libraryStore.ts
// Module 12 — Library Store with Filters & Search

import { writable, derived, get, type Writable } from 'svelte/store'
import { invoke } from '@tauri-apps/api/core'
import type { Game } from '$lib/types'

// ── Steam tool/runtime filter ─────────────────────────────────
// These patterns match names of Steam compatibility tools,
// runtimes, and redistributables that should be hidden from the UI.
const STEAM_TOOL_PATTERNS = [
  /^proton\s/i,
  /^proton-/i,
  /steam linux runtime/i,
  /steamworks common/i,
  /^steam client/i,
  /proton easyantiCheat runtime/i,
  /proton battleye runtime/i,
]

function isSteamTool(name: string): boolean {
  return STEAM_TOOL_PATTERNS.some(p => p.test(name))
}

// ── Raw data ──────────────────────────────────────────────────
export interface GamesStore extends Writable<Game[]> {
  addGame: (game: Game) => void;
  updateGame: (id: string, partial: Partial<Game>) => void;
  removeGame: (id: string) => void;
}

function createGamesStore(): GamesStore {
  const { subscribe, set, update } = writable<Game[]>([])
  return {
    subscribe,
    set,
    update,
    addGame: (game: Game) => update(gs => [...gs, game]),
    updateGame: (id: string, partial: Partial<Game>) => update(gs => gs.map(g => g.id === id ? { ...g, ...partial } : g)),
    removeGame: (id: string) => update(gs => gs.filter(g => g.id !== id))
  }
}
export const games = createGamesStore()
export const isLoading = writable<boolean>(false)

// ── Search ────────────────────────────────────────────────────
export const searchQuery = writable<string>('')

// ── Filters ───────────────────────────────────────────────────
export const activeFilters = writable<string[]>([])

// ── Sort ──────────────────────────────────────────────────────
export type SortKey = 'name' | 'hours' | 'last_played' | 'added'
export const sortKey = writable<SortKey>('name')
export const sortDir = writable<'asc' | 'desc'>('asc')

// ── Derived: filtered + sorted view ───────────────────────────
export const filteredGames = derived(
  [games, searchQuery, activeFilters, sortKey, sortDir],
  ([$games, $query, $filters, $sort, $dir]) => {
    // Filter out Steam tools/runtimes first
    let result = $games.filter(g => !isSteamTool(g.name))

    // 1. Search filter (name match, case-insensitive)
    if ($query.trim()) {
      const q = $query.toLowerCase()
      result = result.filter(g => g.name.toLowerCase().includes(q))
    }

    // 2. Active filters (AND logic — game must satisfy all)
    if ($filters.length > 0) {
      const now = Math.floor(Date.now() / 1000)
      result = result.filter(g =>
        $filters.every(f => {
          switch (f) {
            case 'owned':     return true  // all library games are owned
            case 'installed': return g.is_installed
            case 'custom':    return g.source === 'custom'
            case 'recent':    return g.last_played_at > now - 30 * 24 * 3600
            case 'favourite': return g.is_favourite
            default:          return true
          }
        })
      )
    }

    // 3. Sort
    result = [...result].sort((a, b) => {
      let cmp = 0
      switch ($sort) {
        case 'name':        cmp = a.name.localeCompare(b.name); break
        case 'hours':       cmp = (a.wither_playtime_s + (a.steam_playtime_s || 0)) - (b.wither_playtime_s + (b.steam_playtime_s || 0)); break
        case 'last_played': cmp = a.last_played_at - b.last_played_at; break
        case 'added':       cmp = a.added_at - b.added_at; break
      }
      return $dir === 'asc' ? cmp : -cmp
    })

    return result
  }
)

// ── Actions ───────────────────────────────────────────────────

export async function loadGames(): Promise<void> {
  isLoading.set(true)
  try {
    const result = await invoke<Game[]>('get_all_games')
    // Filter out Steam tools/runtimes at load time so they never appear in any view
    games.set(result.filter(g => !isSteamTool(g.name)))
  } finally {
    isLoading.set(false)
  }
}

export async function addCustomGame(payload: { executable_path: string, name?: string }): Promise<void> {
  const game = await invoke<Game>('add_custom_game', {
    executablePath: payload.executable_path,
    name: payload.name ?? null,
    launchArgs: null
  })
  games.update(gs => [...gs, game])
}

export async function removeGame(gameId: string): Promise<void> {
  await invoke('remove_game', { gameId })
  games.update(gs => gs.filter(g => g.id !== gameId))
}

export async function toggleFavourite(gameId: string): Promise<void> {
  await invoke('toggle_favourite', { game_id: gameId })
  games.update(gs =>
    gs.map(g => g.id === gameId ? { ...g, is_favourite: !g.is_favourite } : g)
  )
}

// Reload library after a sync completes
export async function refreshAfterSync(): Promise<void> {
  await loadGames()
}
