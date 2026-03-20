<script lang="ts">
  import SearchBar from './SearchBar.svelte';
  import FilterDropdown from './FilterDropdown.svelte';
  import { addCustomGame } from '$lib/api';
  import { open } from '@tauri-apps/plugin-dialog';
  import { games, ui, activeFilters } from '$lib/stores';

  async function handleAddGame() {
    try {
      const filePath = await open({
        title: 'Select Game Executable',
        multiple: false,
        filters: [{
          name: 'All Files',
          extensions: ['*']
        }]
      });

      if (filePath) {
        ui.setLoading(true);
        const game = await addCustomGame({
          executable_path: filePath as string
        });
        games.addGame(game);
        ui.setLoading(false);
      }
    } catch (error) {
      console.error('Failed to add game:', error);
      alert('Failed to add game: ' + error);
      ui.setLoading(false);
    }
  }
</script>

<header class="topbar">
  {#if $ui.activeRoute !== '/store'}
    <SearchBar />
  {/if}
  
  <FilterDropdown />
  
  <div class="tb-divider"></div>
  
  <button class="tb-add-btn" onclick={handleAddGame}>
    + Add game
  </button>
</header>

<style>
  .topbar {
    height: 54px;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border-1);
    background: var(--bg-topbar);
    position: relative;
    z-index: 100;
  }

  .tb-divider {
    width: 1px;
    height: 24px;
    background: var(--border-1);
  }

  .tb-add-btn {
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    color: var(--text-primary);
    padding: 7px 16px;
    border-radius: var(--radius-pill);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .tb-add-btn:hover {
    background: var(--bg-s2);
    border-color: var(--border-2);
  }
</style>
