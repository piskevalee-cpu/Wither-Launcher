<script lang="ts">
  import { games, ui } from '$lib/stores';
  import GameCard from '$lib/components/GameCard.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';

  let searchQuery = $derived($ui.searchQuery.toLowerCase());
  
  let filteredGames = $derived(
    $games
      .filter(game => game.name.toLowerCase().includes(searchQuery))
      .sort((a, b) => a.name.localeCompare(b.name))
  );
</script>

<div class="library-page">
  <SectionHeader title="Library" />

  {#if filteredGames.length > 0}
    <div class="card-grid">
      {#each filteredGames as game (game.id)}
        <GameCard {game} />
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      <p class="text-mono text-secondary">No games found.</p>
      {#if searchQuery}
        <p class="text-mono text-tertiary text-sm">Try a different search term</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .library-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-4);
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    text-align: center;
  }
</style>
