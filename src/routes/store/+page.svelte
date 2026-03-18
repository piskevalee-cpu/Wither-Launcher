<script lang="ts">
  import { onMount } from 'svelte';
  import { 
    featured, 
    topSellers, 
    newReleases, 
    specials, 
    loadFeatured, 
    loadCategories,
    isLoading,
    storeError as error
  } from '$lib/stores';
  import { games } from '$lib/stores';
  import StoreCard from '$lib/components/StoreCard.svelte';

  let activeTab = $state('featured');

  onMount(async () => {
    await loadFeatured();
    await loadCategories();
  });

  function isInLibrary(appId: number): boolean {
    return $games.some(g => g.steam_app_id === appId);
  }
</script>

<div class="store-page">
  <div class="store-header">
    <h1 class="page-title text-mono text-xl text-primary">Steam Store</h1>
    
    <div class="store-tabs">
      <button 
        class="tab" 
        class:active={activeTab === 'featured'}
        onclick={() => activeTab = 'featured'}
      >
        Featured
      </button>
      <button 
        class="tab" 
        class:active={activeTab === 'topseller'}
        onclick={() => activeTab = 'topseller'}
      >
        Top Sellers
      </button>
      <button 
        class="tab" 
        class:active={activeTab === 'newrelease'}
        onclick={() => activeTab = 'newrelease'}
      >
        New Releases
      </button>
      <button 
        class="tab" 
        class:active={activeTab === 'specials'}
        onclick={() => activeTab = 'specials'}
      >
        Specials
      </button>
    </div>
  </div>

  {#if $isLoading}
    <div class="loading-state">
      <p class="text-mono text-secondary">Loading store...</p>
    </div>
  {:else if $error}
    <div class="error-state">
      <p class="text-mono text-secondary">Error loading store</p>
      <p class="text-mono text-tertiary text-sm">{$error}</p>
    </div>
  {:else}
    <div class="store-content">
      {#if activeTab === 'featured'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">Featured Games</h2>
          <div class="store-grid">
            {#each $featured as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'topseller'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">Top Sellers</h2>
          <div class="store-grid">
            {#each $topSellers as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'newrelease'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">New Releases</h2>
          <div class="store-grid">
            {#each $newReleases as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'specials'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">Specials</h2>
          <div class="store-grid">
            {#each $specials as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} />
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .store-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding: var(--space-6);
    max-width: 1400px;
    margin: 0 auto;
  }

  .store-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .page-title {
    font-weight: 600;
  }

  .store-tabs {
    display: flex;
    gap: var(--space-2);
    border-bottom: 1px solid var(--border-1);
    padding-bottom: var(--space-2);
  }

  .tab {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 8px 16px;
    border-radius: var(--radius-pill);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab:hover {
    background: var(--bg-s2);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--text-primary);
    color: #000;
  }

  .store-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .store-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .section-title {
    font-weight: 500;
  }

  .store-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--space-4);
  }

  .loading-state, .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-10);
    gap: var(--space-2);
  }

  .error-state {
    color: var(--status-error);
  }
</style>
