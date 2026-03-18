// src/lib/stores/syncStore.ts
import { writable, derived } from 'svelte/store'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { SyncResult } from '$lib/types'

export type SyncState = 'disconnected' | 'syncing' | 'synced' | 'error'

export const syncState = writable<SyncState>('disconnected')
export const lastSyncedAt = writable<number | null>(null)
export const syncError = writable<string | null>(null)
export const syncResult = writable<SyncResult | null>(null)

// Human-readable status label for sidebar display
export const syncLabel = derived(
  [syncState, lastSyncedAt],
  ([$state, $last]) => {
    switch ($state) {
      case 'syncing':      return 'Syncing...'
      case 'synced':       return $last ? 'Synced' : 'Synced'
      case 'error':        return 'Sync failed'
      case 'disconnected': return 'Not connected'
    }
  }
)

// Called once on app mount
export async function runStartupSync(): Promise<void> {
  // Check if Steam credentials exist
  const apiKey = await invoke<string>('get_setting', { key: 'steam_api_key' })
  if (!apiKey) {
    syncState.set('disconnected')
    return
  }

  syncState.set('syncing')
  syncError.set(null)

  try {
    const result = await invoke<SyncResult>('sync_steam')
    syncResult.set(result)
    lastSyncedAt.set(result.synced_at)
    syncState.set('synced')

    // If sync had non-fatal errors, still show synced but log them
    if (result.errors.length > 0) {
      console.warn('[wither] Sync completed with warnings:', result.errors)
    }
  } catch (e: unknown) {
    syncState.set('error')
    syncError.set(String(e))
  }
}

// Listen for backend-emitted sync events (from file watcher / periodic sync)
export async function initSyncListener(): Promise<void> {
  await listen<SyncResult>('sync_completed', (event) => {
    syncResult.set(event.payload)
    lastSyncedAt.set(event.payload.synced_at)
    syncState.set('synced')
  })
}
