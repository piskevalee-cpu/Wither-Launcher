<script lang="ts">
  import { games } from '$lib/stores';
  import RecentCard from '$lib/components/RecentCard.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';

  let recentlyPlayed = $derived(
    $games
      .filter((g) => g.last_played_at > 0)
      .sort((a, b) => b.last_played_at - a.last_played_at)
  );
</script>

<div class="recent-page">
  <SectionHeader title="Recently Played" />
  
  {#if recentlyPlayed.length > 0}
    <div class="recent-list">
      {#each recentlyPlayed as game (game.id)}
        <RecentCard {game} />
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      <p class="text-mono text-secondary">No recently played games.</p>
      <p class="text-mono text-tertiary text-sm">
        Launch a game to see it here.
      </p>
    </div>
  {/if}
</div>

<style>
  .recent-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    text-align: center;
  }

  .empty-state p {
    margin: var(--space-2) 0;
  }
</style>
