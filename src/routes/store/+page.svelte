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
    storeError as error,
    searchResults,
    searchGames
  } from '$lib/stores';
  import { games } from '$lib/stores';
  import StoreCard from '$lib/components/StoreCard.svelte';
  import StoreGameModal from '$lib/components/StoreGameModal.svelte';

  let activeTab = $state('featured');
  let localSearchQuery = $state('');
  let searchTimeout: ReturnType<typeof setTimeout>;
  let selectedStoreAppId = $state<number | null>(null);

  function handleSearchInput(e: Event) {
    const target = e.target as HTMLInputElement;
    localSearchQuery = target.value;
    
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      searchGames(localSearchQuery);
      if (localSearchQuery.trim().length > 0) {
        activeTab = 'search';
      } else {
        if (activeTab === 'search') activeTab = 'featured';
      }
    }, 500);
  }

  function handleSelectGame(appId: number) {
    selectedStoreAppId = appId;
  }

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
    
    <div class="store-search-container">
      <svg class="search-icon" viewBox="0 0 13 13" fill="none">
        <circle cx="5.5" cy="5.5" r="4" stroke="rgba(255,255,255,0.22)" stroke-width="1.3"/>
        <path d="M9 9l2.8 2.8" stroke="rgba(255,255,255,0.22)" stroke-width="1.3" stroke-linecap="round"/>
      </svg>
      <input 
        type="text" 
        placeholder="Search Steam Store..." 
        value={localSearchQuery}
        oninput={handleSearchInput}
        class="store-search-input"
      />
      {#if localSearchQuery}
        <button class="clear-search-button" onclick={() => { localSearchQuery = ''; searchGames(''); if (activeTab === 'search') activeTab = 'featured'; }}>
          <svg viewBox="0 0 9 9" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M1.5 4.5l2 2 4-4" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
      {/if}
    </div>

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
              <StoreCard {item} inLibrary={isInLibrary(item.id)} onSelect={handleSelectGame} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'topseller'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">Top Sellers</h2>
          <div class="store-grid">
            {#each $topSellers as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} onSelect={handleSelectGame} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'newrelease'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">New Releases</h2>
          <div class="store-grid">
            {#each $newReleases as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} onSelect={handleSelectGame} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'specials'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">Specials</h2>
          <div class="store-grid">
            {#each $specials as item (item.id)}
              <StoreCard {item} inLibrary={isInLibrary(item.id)} onSelect={handleSelectGame} />
            {/each}
          </div>
        </section>
      {:else if activeTab === 'search'}
        <section class="store-section">
          <h2 class="section-title text-mono text-base text-primary">Search Results for "{localSearchQuery}"</h2>
          {#if $isLoading}
             <p class="text-mono text-secondary">Searching...</p>
          {:else if $searchResults.length === 0}
             <p class="text-mono text-secondary">No games found.</p>
          {:else}
            <div class="store-grid">
              {#each $searchResults as item (item.id)}
                <StoreCard {item} inLibrary={isInLibrary(item.id)} onSelect={handleSelectGame} />
              {/each}
            </div>
          {/if}
        </section>
      {/if}
    </div>
  {/if}
</div>

{#if selectedStoreAppId}
  <StoreGameModal appId={selectedStoreAppId} onClose={() => selectedStoreAppId = null} />
{/if}

<style>
  .store-search-container {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-pill);
    padding: 7px 16px;
    margin-bottom: var(--space-2);
    transition: border-color 0.15s, background 0.15s;
    width: 100%;
    max-width: 400px;
  }
  .store-search-container:focus-within {
    border-color: var(--border-2);
    background: var(--bg-s2);
  }
  .search-icon {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
  }
  .store-search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-family: var(--font-sans);
    font-size: var(--text-base);
    color: var(--text-primary);
    width: 100%;
  }
  .store-search-input::placeholder { color: var(--text-tertiary); }
  .clear-search-button {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
  }
  .clear-search-button:hover {
    background: var(--bg-s2);
    color: var(--text-primary);
  }
  .clear-search-button svg { width: 9px; height: 9px; }

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
