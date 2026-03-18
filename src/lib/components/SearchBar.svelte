<script lang="ts">
  import { ui } from '$lib/stores';

  let localQuery = $state('');

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    localQuery = target.value;
    ui.setSearchQuery(localQuery);
  }

  function clearSearch() {
    localQuery = '';
    ui.setSearchQuery('');
  }
</script>

<div class="search-bar">
  <svg class="search-icon" viewBox="0 0 13 13" fill="none">
    <circle cx="5.5" cy="5.5" r="4" stroke="rgba(255,255,255,0.22)" stroke-width="1.3"/>
    <path d="M9 9l2.8 2.8" stroke="rgba(255,255,255,0.22)" stroke-width="1.3" stroke-linecap="round"/>
  </svg>
  <input 
    type="text" 
    placeholder="Search games…" 
    value={localQuery}
    oninput={handleInput}
    class="search-input"
  />
  {#if localQuery}
    <button class="clear-button" onclick={clearSearch} aria-label="Clear search">
      <svg viewBox="0 0 9 9" fill="none" stroke="currentColor" stroke-width="1.6">
        <path d="M1.5 4.5l2 2 4-4" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
  {/if}
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    border-radius: var(--radius-pill);
    padding: 7px 16px;
    flex: 1;
    max-width: 380px;
    transition: border-color 0.15s, background 0.15s;
  }

  .search-bar:focus-within {
    border-color: var(--border-2);
    background: var(--bg-s2);
  }

  .search-icon {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-family: var(--font-sans);
    font-size: var(--text-base);
    color: var(--text-primary);
    width: 100%;
  }

  .search-input::placeholder {
    color: var(--text-tertiary);
  }

  .clear-button {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    transition: all 0.12s ease;
  }

  .clear-button:hover {
    background: var(--bg-s2);
    color: var(--text-primary);
  }

  .clear-button svg {
    width: 9px;
    height: 9px;
  }
</style>
