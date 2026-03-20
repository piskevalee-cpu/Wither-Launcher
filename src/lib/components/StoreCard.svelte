<script lang="ts">
  import type { FeaturedItem, SearchItem } from '$lib/stores/storeStore';
  import { launchGame } from '$lib/api';
  import { games, isInLibrary, openInSteamStore } from '$lib/stores';

  let { item, inLibrary = false, onSelect }: { item: FeaturedItem | SearchItem, inLibrary?: boolean, onSelect?: (id: number) => void } = $props();
  let isLaunching = $state(false);

  // Format price with currency
  function formatPrice(item: FeaturedItem | SearchItem): string {
    if ('final_price' in item) {
      // FeaturedItem
      if (!item.final_price) return 'Free';
      const price = item.final_price / 100;
      const currency = item.currency || '€';
      return `${currency}${price.toFixed(2)}`;
    } else {
      // SearchItem
      if (item.sale_price) return item.sale_price;
      if (item.price) return item.price;
      return 'Free';
    }
  }

  function getOriginalPrice(item: FeaturedItem | SearchItem): string | null {
    if ('original_price' in item && item.original_price) {
      const price = item.original_price / 100;
      const currency = item.currency || '€';
      return `${currency}${price.toFixed(2)}`;
    }
    return null;
  }

  async function handleLaunch() {
    if (isLaunching) return;
    isLaunching = true;
    try {
      // For store games, we need to find the game in library first
      const libraryGames = $games;
      const game = libraryGames.find(g => g.steam_app_id === item.id);
      if (game) {
        await launchGame(game.id);
      }
    } catch (error) {
      console.error('Failed to launch game:', error);
    } finally {
      isLaunching = false;
    }
  }

  function handleViewDetails() {
    if (onSelect) {
      onSelect(item.id);
    } else {
      openInSteamStore(item.id);
    }
  }
</script>

<div class="store-card">
  <div class="card-image">
    <img src={'header_image' in item ? item.header_image : (item.logo || '/placeholder-cover.jpg')} alt={item.name} />
    
    {#if 'discount_percent' in item && item.discounted && item.discount_percent}
      <div class="discount-badge">-{item.discount_percent}%</div>
    {/if}
    
    {#if inLibrary}
      <div class="in-library-badge">In Library</div>
    {/if}
  </div>
  
  <div class="card-info">
    <h3 class="card-title">{item.name}</h3>
    
    <div class="card-price">
      {#if 'discount_percent' in item && item.discounted && getOriginalPrice(item)}
        <span class="original-price">{getOriginalPrice(item)}</span>
      {/if}
      <span class="final-price">{formatPrice(item)}</span>
    </div>
    
    <div class="card-actions">
      {#if inLibrary}
        <button class="play-btn" onclick={handleLaunch} disabled={isLaunching}>
          {#if isLaunching}
            ...
          {:else}
            ▶ Play
          {/if}
        </button>
      {:else}
        <button class="view-btn" onclick={handleViewDetails}>
          View Details
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .store-card {
    display: flex;
    flex-direction: column;
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: background 0.15s ease, transform 0.15s ease;
  }

  .store-card:hover {
    background: var(--bg-s2);
    transform: translateY(-2px);
  }

  .card-image {
    position: relative;
    aspect-ratio: 16/9;
    overflow: hidden;
  }

  .card-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .discount-badge {
    position: absolute;
    top: 8px;
    right: 8px;
    background: #5c7e10;
    color: #fff;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
  }

  .in-library-badge {
    position: absolute;
    top: 8px;
    left: 8px;
    background: var(--status-synced);
    color: #000;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 600;
  }

  .card-info {
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .card-title {
    font-family: var(--font-mono);
    font-size: var(--text-base);
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .card-price {
    display: flex;
    gap: 6px;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .original-price {
    color: var(--text-tertiary);
    text-decoration: line-through;
  }

  .final-price {
    color: var(--text-primary);
    font-weight: 600;
  }

  .card-actions {
    margin-top: auto;
    padding-top: var(--space-2);
  }

  .play-btn, .view-btn {
    width: 100%;
    background: rgba(255, 255, 255, 0.15);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: var(--text-primary);
    padding: 6px 12px;
    border-radius: var(--radius-pill);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .play-btn:hover:not(:disabled), .view-btn:hover {
    background: rgba(255, 255, 255, 0.25);
    border-color: rgba(255, 255, 255, 0.4);
  }

  .play-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
