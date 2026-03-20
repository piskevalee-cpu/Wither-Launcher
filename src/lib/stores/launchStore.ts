// src/lib/stores/launchStore.ts
// Module 12 — Launch State Management

import { writable, derived } from 'svelte/store'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type LaunchStatus =
  | 'idle'
  | 'starting_steam'
  | 'waiting_for_steam'
  | 'launching_game'
  | 'running'
  | 'exited'
  | 'error'

export interface LaunchState {
  status:     LaunchStatus
  game_id:    string | null
  session_id: string | null
  started_at: number | null
  duration_s: number | null
  error:      string | null
}

const INITIAL: LaunchState = {
  status: 'idle',
  game_id: null,
  session_id: null,
  started_at: null,
  duration_s: null,
  error: null
}

export const launchState = writable<LaunchState>(INITIAL)

export function resetLaunchState() {
  launchState.set(INITIAL)
}

// Human-readable status messages for the UI
export const STATUS_LABELS: Record<LaunchStatus, string> = {
  idle:              '',
  starting_steam:    'Starting Steam...',
  waiting_for_steam: 'Initializing...',
  launching_game:    'Loading game...',
  running:           'Running',
  exited:            'Session saved',
  error:             'Launch failed',
}

// Derived: is any game currently running?
export const isGameRunning = derived(
  launchState,
  $s => $s.status === 'running'
)

// Live elapsed timer — ticks every second while a game is running
export const elapsedSeconds = writable(0)

let _timerInterval: ReturnType<typeof setInterval> | null = null

function startTimer(started_at: number) {
  stopTimer()
  _timerInterval = setInterval(() => {
    const secs = Math.floor(Date.now() / 1000) - started_at
    elapsedSeconds.set(secs)
  }, 1000)
  // Set initial value immediately
  elapsedSeconds.set(Math.floor(Date.now() / 1000) - started_at)
}

function stopTimer() {
  if (_timerInterval) {
    clearInterval(_timerInterval)
    _timerInterval = null
  }
  elapsedSeconds.set(0)
}

// Subscribe to backend launch state events
export async function initLaunchListener(): Promise<void> {
  await listen<any>('game_launch_state', (event) => {
    const payload = event.payload
    launchState.set({ ...INITIAL, ...payload })

    // Manage the live timer
    if (payload.status === 'running' && payload.started_at) {
      startTimer(payload.started_at)
    } else if (payload.status !== 'running') {
      stopTimer()
    }

    // Auto-reset to idle 3s after game exits
    if (payload.status === 'exited') {
      setTimeout(() => {
        launchState.set(INITIAL)
      }, 3000)
    }
  })
}

export async function launchGame(gameId: string): Promise<void> {
  launchState.set({ ...INITIAL, status: 'starting_steam', game_id: gameId })
  try {
    await invoke('launch_game', { game_id: gameId })
    // Further state updates come from backend events via initLaunchListener
  } catch (e) {
    launchState.set({ 
      ...INITIAL, 
      status: 'error', 
      game_id: gameId, 
      error: String(e) 
    })
  }
}
