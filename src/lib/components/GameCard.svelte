<script lang="ts">
  import type { Game } from '$lib/types';
  import { launchGame } from '$lib/api';
  import { games, ui, launchState, isGameRunning, elapsedSeconds, STATUS_LABELS, removeGame, resetLaunchState } from '$lib/stores';
  import EditGameModal from './EditGameModal.svelte';

  let { game, isMostRecent = false } = $props();
  let isLaunching = $state(false);
  let showEditModal = $state(false);

  // Check if this game is currently running
  let runningGameId = $derived($launchState.game_id);
  let isThisGameRunning = $derived(runningGameId === game.id && $launchState.status === 'running');
  let elapsed = $derived(isThisGameRunning ? $elapsedSeconds : 0);

  // Check if this game is in an intermediate launch state (spinner states)
  let isThisGameLaunching = $derived(
    runningGameId === game.id && 
    ['starting_steam', 'waiting_for_steam', 'launching_game'].includes($launchState.status)
  );
  let launchStatusLabel = $derived(
    isThisGameLaunching ? STATUS_LABELS[$launchState.status] : ''
  );
  
  // Reset isLaunching when game actually starts running or exits
  $effect(() => {
    const status = $launchState.status;
    const gameId = $launchState.game_id;
    if (gameId === game.id && (status === 'running' || status === 'exited' || status === 'error')) {
      isLaunching = false;
    }
  });
  
  // Format elapsed time as HH:MM:SS
  function formatElapsed(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    if (h > 0) return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  // Convert file path to web-accessible URL
  function getCoverDisplayUrl(path: string | null): string {
    if (!path) return '/placeholder-cover.jpg';
    if (path.startsWith('data:')) return path;
    if (path.startsWith('http://') || path.startsWith('https://')) return path;
    return '/placeholder-cover.jpg';
  }

  const coverUrl = $state(getCoverDisplayUrl(game.cover_url));
  let coverFailed = $state(false);
  
  function handleCoverError() {
    if (!coverFailed && game.steam_app_id) {
      coverFailed = true;
      const headerUrl = `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.steam_app_id}/header.jpg`;
      const img = document.querySelector(`[data-game-id="${game.id}"] img`) as HTMLImageElement;
      if (img) img.src = headerUrl;
    }
  }

  // Calculate total playtime (Wither + Steam) in hours and minutes
  const totalPlaytimeSeconds = (game.wither_playtime_s || 0) + (game.steam_playtime_s || 0);
  const playtimeHours = Math.floor(totalPlaytimeSeconds / 3600);
  const playtimeMinutes = Math.floor((totalPlaytimeSeconds % 3600) / 60);
  const playtimeDisplay = playtimeHours > 0 
    ? `${playtimeHours}h ${playtimeMinutes}m` 
    : `${playtimeMinutes}m`;
  
  const isContextMenuOpen = $derived($ui.contextMenuGameId === game.id);

  async function handleLaunch() {
    if (isLaunching || isThisGameRunning || isThisGameLaunching) return;
    isLaunching = true;
    try {
      await launchGame(game.id);
    } catch (error) {
      console.error('Failed to launch game:', error);
      isLaunching = false;
      // If launch fails, reset global status so it doesn't stay stuck on "Loading game..."
      if ($launchState.game_id === game.id) {
        resetLaunchState();
      }
    }
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    ui.setContextMenuGameId(game.id);
  }

  function closeContextMenu() {
    ui.setContextMenuGameId(null);
  }

  async function handleRemove() {
    if (confirm(`Remove "${game.name}" from your library?`)) {
      try {
        await removeGame(game.id);
      } catch (error) {
        console.error('Failed to remove game:', error);
        alert('Failed to remove game: ' + error);
      }
    }
    closeContextMenu();
  }

  function handleEdit() {
    showEditModal = true;
    closeContextMenu();
  }

  // Close context menu when clicking outside
  $effect(() => {
    function handleClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      if (!target.closest('.game-card')) {
        closeContextMenu();
      }
    }
    document.addEventListener('click', handleClick);
    return () => document.removeEventListener('click', handleClick);
  });
</script>

<div class="game-card" data-game-id={game.id} oncontextmenu={handleContextMenu}>
  <div class="card-image">
    <img src={coverUrl} alt={game.name} onerror={handleCoverError} />
    <div class="card-overlay" class:overlay-visible={isThisGameLaunching || isThisGameRunning}>
      <button class="play-button" onclick={handleLaunch} disabled={isLaunching || isThisGameRunning || isThisGameLaunching}>
        {#if isThisGameLaunching}
          <span class="spinner"></span> {launchStatusLabel}
        {:else if isLaunching}
          <span class="spinner"></span> Launching...
        {:else if isThisGameRunning}
          <span class="running-dot"></span> {formatElapsed(elapsed)}
        {:else if game.source === 'steam' && !game.is_installed}
          ⬇ Install
        {:else}
          ▶ Play
        {/if}
      </button>
    </div>
    {#if isMostRecent && game.last_played_at > 0}
      <div class="last-played-badge text-mono text-xs">
        Last played
      </div>
    {/if}
    {#if isThisGameRunning}
      <div class="running-badge text-mono text-xs">
        ● Running
      </div>
    {/if}
    <div class="source-badge text-mono text-xs">
      {game.source === 'steam' ? 'Steam' : 'Custom'}
    </div>
    
    {#if isContextMenuOpen}
      <div class="context-menu">
        {#if game.source === 'custom'}
          <button class="context-item" onclick={handleEdit}>
            ✏️ Edit
          </button>
        {/if}
        <button class="context-item" onclick={handleRemove}>
          🗑 Remove from Library
        </button>
      </div>
    {/if}
  </div>
  <div class="card-info">
    <h3 class="card-title text-mono text-base text-primary">{game.name}</h3>
    {#if game.genre}
      <p class="card-genre text-sans text-sm text-secondary">{game.genre}</p>
    {/if}
    <div class="card-stats text-mono text-xs text-tertiary">
      <span>{playtimeDisplay} played</span>
      {#if game.steam_playtime_s > 0}
        <span class="steam-playtime" title="Steam playtime">(+{Math.floor(game.steam_playtime_s / 3600)}h {Math.floor((game.steam_playtime_s % 3600) / 60)}m Steam)</span>
      {/if}
    </div>
    <p class="card-id text-mono text-xs text-tertiary" title={game.id}>
      {#if game.steam_app_id}
        AppID: {game.steam_app_id}
      {:else}
        ID: {game.id.substring(0, 12)}…
      {/if}
    </p>
  </div>
</div>

<style>
  .game-card {
    display: flex;
    flex-direction: column;
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: transform 0.25s ease;
  }

  .game-card:hover {
    transform: scale(1.02);
  }

  .card-image {
    position: relative;
    aspect-ratio: 3/4;
    overflow: hidden;
  }

  .card-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .card-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .game-card:hover .card-overlay,
  .card-overlay.overlay-visible {
    opacity: 1;
  }

  .play-button {
    background: rgba(255, 255, 255, 0.15);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: var(--text-primary);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-pill);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .play-button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.25);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .play-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Spinner for intermediate launch states */
  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Green dot for running state */
  .running-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    background: var(--status-synced, #4ade80);
    border-radius: 50%;
    animation: pulse-dot 2s ease-in-out infinite;
  }

  @keyframes pulse-dot {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .last-played-badge {
    position: absolute;
    bottom: var(--space-2);
    left: var(--space-2);
    background: var(--bg-s3);
    color: var(--text-primary);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
  }

  .running-badge {
    position: absolute;
    bottom: var(--space-2);
    right: 40px;
    background: var(--status-synced);
    color: #000;
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    font-weight: 600;
  }

  .source-badge {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    background: rgba(0, 0, 0, 0.8);
    color: var(--color-text-secondary);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
  }

  .context-menu {
    position: absolute;
    top: var(--space-2);
    left: var(--space-2);
    background: var(--color-bg-3);
    border: 1px solid var(--color-border-1);
    border-radius: var(--radius-md);
    padding: var(--space-2);
    z-index: 10;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }

  .context-item {
    background: transparent;
    border: none;
    color: var(--color-text-primary);
    padding: var(--space-2) var(--space-3);
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .context-item:hover {
    background: var(--color-accent);
  }

  .card-info {
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .card-title {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-stats {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .steam-playtime {
    font-size: 10px;
    color: var(--color-accent);
  }

  .card-id {
    margin-top: 2px;
    opacity: 0.5;
    font-size: 9px;
  }
</style>

{#if showEditModal}
  <EditGameModal {game} onClose={() => showEditModal = false} />
{/if}
