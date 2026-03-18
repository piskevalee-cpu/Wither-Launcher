// src/lib/stores/libraryStore.ts
// Module 12 — Library Store with Filters & Search

import { writable, derived, get } from 'svelte/store'
import { invoke } from '@tauri-apps/api/core'
import type { Game } from '$lib/types'

// ── Raw data ──────────────────────────────────────────────────
export const games = writable<Game[]>([])
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
    let result = $games

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
    games.set(result)
  } finally {
    isLoading.set(false)
  }
}

export async function addCustomGame(payload: { executable_path: string, name?: string }): Promise<void> {
  const game = await invoke<Game>('add_custom_game', payload)
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
