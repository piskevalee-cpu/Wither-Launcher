<script lang="ts">
  import { onMount } from 'svelte';
  import { currentAppDetails, loadAppDetails, isLoading, isInLibrary, games, openInSteamStore } from '$lib/stores';
  import { launchGame } from '$lib/api';

  let { appId, onClose }: { appId: number, onClose: () => void } = $props();

  let isLaunching = $state(false);
  let showSpinner = $state(true); // show spinner while loading

  onMount(async () => {
    showSpinner = true;
    await loadAppDetails(appId);
    showSpinner = false;
  });

  // Derived state to check if game is in library
  let inLibraryStatus = $derived(isInLibrary(appId, $games));
  let libraryGame = $derived($games.find(g => g.steam_app_id === appId));

  async function handleLaunch() {
    if (isLaunching || !libraryGame) return;
    isLaunching = true;
    try {
      await launchGame(libraryGame.id);
      onClose(); // Optional: close modal when game launches
    } catch (error) {
      console.error('Failed to launch game:', error);
    } finally {
      isLaunching = false;
    }
  }

  function handleOpenSteam() {
    openInSteamStore(appId);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  $effect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  });
</script>

<div class="modal-overlay" onclick={onClose}>
  <div class="modal-content" onclick={(e) => e.stopPropagation()}>
    <button class="close-button" onclick={onClose}>×</button>
    
    {#if showSpinner || $isLoading}
      <div class="loading-state">
        <div class="spinner"></div>
        <p class="text-mono text-secondary">Loading details...</p>
      </div>
    {:else if $currentAppDetails && $currentAppDetails.steam_appid === appId}
      {@const app = $currentAppDetails}
      <div class="modal-header-image">
        <img src={app.header_image || '/placeholder-cover.jpg'} alt={app.name} />
      </div>
      
      <div class="modal-body">
        <div class="title-row">
          <h2 class="game-title text-mono">{app.name}</h2>
          <div class="price-badge">
            {#if app.is_free}
              Free
            {:else if app.price_overview}
              {#if app.price_overview.discount_percent > 0}
                <span class="discount">-{app.price_overview.discount_percent}%</span>
                <span class="original">{app.price_overview.initial_formatted}</span>
              {/if}
              <span class="final">{app.price_overview.final_formatted}</span>
            {/if}
          </div>
        </div>

        <div class="meta-info">
          {#if app.developers && app.developers.length > 0}
            <span class="meta-item"><strong>Developer:</strong> {app.developers.join(', ')}</span>
          {/if}
          {#if app.release_date && app.release_date.date}
            <span class="meta-item"><strong>Release:</strong> {app.release_date.date}</span>
          {/if}
        </div>

        <div class="description text-sans text-secondary">
          {#if app.short_description}
            {@html app.short_description}
          {/if}
        </div>

        {#if app.genres && app.genres.length > 0}
          <div class="genres">
            {#each app.genres as genre}
              <span class="genre-tag">{genre.description}</span>
            {/each}
          </div>
        {/if}
      </div>
      
      <div class="modal-footer">
        {#if inLibraryStatus && libraryGame}
          <button class="action-button play-btn" onclick={handleLaunch} disabled={isLaunching}>
            {#if isLaunching}
              Launching...
            {:else if !libraryGame.is_installed}
              ⬇ Install
            {:else}
              ▶ Play
            {/if}
          </button>
        {:else}
          <button class="action-button steam-btn" onclick={handleOpenSteam}>
            Open in Steam
          </button>
        {/if}
      </div>
    {:else}
      <div class="error-state">
        <p class="text-mono text-secondary">Failed to load game details.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }
  
  .modal-content {
    position: relative;
    background: var(--color-bg-2, var(--bg-s1, #1a1a1a));
    border: 1px solid var(--color-border-1, rgba(255,255,255,0.1));
    border-radius: var(--radius-lg, 12px);
    width: 100%;
    max-width: 600px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
  }
  
  .close-button {
    position: absolute;
    top: 12px;
    right: 12px;
    background: rgba(0, 0, 0, 0.5);
    border: none;
    color: #fff;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    font-size: 20px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
    backdrop-filter: blur(4px);
    transition: background 0.2s;
  }
  
  .close-button:hover {
    background: rgba(0, 0, 0, 0.8);
  }

  .loading-state, .error-state {
    padding: 60px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: var(--color-accent, #007bff);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  
  .modal-header-image {
    width: 100%;
    aspect-ratio: 460 / 215;
    background: #000;
  }
  
  .modal-header-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  
  .modal-body {
    padding: var(--space-5, 20px);
    display: flex;
    flex-direction: column;
    gap: var(--space-4, 16px);
  }
  
  .title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
  }
  
  .game-title {
    font-size: 24px;
    font-weight: 600;
    color: var(--color-text-primary, #fff);
    margin: 0;
    line-height: 1.2;
  }
  
  .price-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255,255,255,0.05);
    padding: 6px 12px;
    border-radius: 6px;
    font-family: var(--font-mono, monospace);
    font-size: 14px;
    white-space: nowrap;
  }
  
  .discount {
    color: #a4d007;
    background: rgba(164, 208, 7, 0.2);
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: bold;
  }
  
  .original {
    color: var(--color-text-tertiary, #888);
    text-decoration: line-through;
    font-size: 12px;
  }
  
  .final {
    color: var(--color-text-primary, #fff);
    font-weight: 600;
  }

  .meta-info {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    font-family: var(--font-sans, sans-serif);
    font-size: 13px;
    color: var(--color-text-secondary, #aaa);
  }

  .meta-item strong {
    color: var(--color-text-primary, #ddd);
    font-weight: 500;
  }
  
  .description {
    font-size: 14px;
    line-height: 1.5;
    color: var(--color-text-secondary, #bbb);
  }
  
  .genres {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 4px;
  }
  
  .genre-tag {
    background: rgba(255,255,255,0.08);
    color: var(--color-text-secondary, #ccc);
    padding: 4px 10px;
    border-radius: 12px;
    font-size: 11px;
    font-family: var(--font-mono, monospace);
  }
  
  .modal-footer {
    padding: var(--space-4, 16px) var(--space-5, 20px);
    border-top: 1px solid var(--color-border-1, rgba(255,255,255,0.1));
    display: flex;
    justify-content: flex-end;
    background: rgba(0,0,0,0.2);
  }
  
  .action-button {
    padding: 10px 24px;
    border-radius: var(--radius-pill, 99px);
    font-family: var(--font-mono, monospace);
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    border: none;
  }
  
  .play-btn {
    background: var(--status-synced, #4ade80);
    color: #000;
  }
  
  .play-btn:hover:not(:disabled) {
    background: #3bca6b;
    transform: translateY(-1px);
  }
  
  .play-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .steam-btn {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2);
  }

  .steam-btn:hover {
    background: rgba(255, 255, 255, 0.25);
    border-color: rgba(255, 255, 255, 0.4);
  }
</style>
