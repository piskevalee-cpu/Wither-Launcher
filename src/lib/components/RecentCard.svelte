<script lang="ts">
  import type { Game } from '$lib/types';
  import { launchGame } from '$lib/api';

  let { game, isMostRecent = false } = $props();
  let isLaunching = $state(false);

  const coverUrl = game.cover_url || '/placeholder-cover.jpg';
  
  // Calculate total playtime (Wither + Steam) in hours and minutes
  const totalPlaytimeSeconds = (game.wither_playtime_s || 0) + (game.steam_playtime_s || 0);
  const playtimeHours = Math.floor(totalPlaytimeSeconds / 3600);
  const playtimeMinutes = Math.floor((totalPlaytimeSeconds % 3600) / 60);
  const playtimeDisplay = playtimeHours > 0 
    ? `${playtimeHours}h ${playtimeMinutes}m` 
    : `${playtimeMinutes}m`;
  
  const lastPlayed = new Date(game.last_played_at * 1000).toLocaleDateString();

  async function handleLaunch() {
    if (isLaunching) return;
    isLaunching = true;
    try {
      await launchGame(game.id);
    } catch (error) {
      console.error('Failed to launch game:', error);
    } finally {
      isLaunching = false;
    }
  }
</script>

<div class="recent-card">
  <div class="card-image">
    <img src={coverUrl} alt={game.name} />
  </div>
  <div class="card-info">
    <h3 class="card-title text-mono text-base text-primary">{game.name}</h3>
    <p class="card-meta text-sans text-sm text-secondary">
      Last played: {lastPlayed}
    </p>
    <p class="card-stats text-mono text-xs text-tertiary">
      {playtimeHours}h {playtimeMinutes}m played
    </p>
  </div>
  <div class="card-overlay">
    <button class="play-button" onclick={handleLaunch} disabled={isLaunching}>
      {#if isLaunching}
        ...
      {:else}
        ▶ Play
      {/if}
    </button>
  </div>
</div>

<style>
  .recent-card {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    background: var(--bg-2);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
    transition: background 0.15s ease;
    overflow: hidden;
  }

  .recent-card:hover {
    background: var(--bg-3);
  }

  .card-image {
    width: 90px;
    height: 90px;
    border-radius: var(--radius-md);
    overflow: hidden;
    flex-shrink: 0;
  }

  .card-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .card-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .card-title {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding-right: var(--space-4);
    opacity: 0;
    transition: opacity 0.15s ease;
    border-radius: var(--radius-lg);
  }

  .recent-card:hover .card-overlay {
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

  .steam-playtime {
    font-size: 10px;
    color: var(--color-accent);
    display: block;
    margin-top: 2px;
  }
</style>
