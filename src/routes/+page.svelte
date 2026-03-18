<script lang="ts">
  import { games, ui } from '$lib/stores';
  import GameCard from '$lib/components/GameCard.svelte';
  import RecentCard from '$lib/components/RecentCard.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';

  let searchQuery = $derived($ui.searchQuery.toLowerCase());

  let favouriteGames = $derived(
    $games
      .filter((g) => g.is_favourite && g.name.toLowerCase().includes(searchQuery))
      .slice(0, 6)
  );
  let recentlyPlayed = $derived(
    $games
      .filter((g) => g.last_played_at > 0 && g.name.toLowerCase().includes(searchQuery))
      .sort((a, b) => b.last_played_at - a.last_played_at)
      .slice(0, 3)
  );
  let allGames = $derived(
    $games
      .filter((g) => g.name.toLowerCase().includes(searchQuery))
      .sort((a, b) => b.last_played_at - a.last_played_at)
      .slice(0, 6)
  );
</script>

<div class="home-page">
  {#if favouriteGames.length > 0}
    <section class="section">
      <SectionHeader title="Favourites" viewAll="/library?favourite=true" />
      <div class="card-grid">
        {#each favouriteGames as game (game.id)}
          <GameCard {game} />
        {/each}
      </div>
    </section>
  {/if}

  {#if recentlyPlayed.length > 0}
    <section class="section">
      <SectionHeader title="Recently Played" viewAll="/recent" />
      <div class="recent-list">
        {#each recentlyPlayed as game, i (game.id)}
          <RecentCard {game} isMostRecent={i === 0} />
        {/each}
      </div>
    </section>
  {/if}

  <section class="section">
    <SectionHeader title="Games" viewAll="/library" />
    <div class="card-grid">
      {#each allGames as game, i (game.id)}
        <GameCard {game} isMostRecent={i === 0 && recentlyPlayed.length > 0} />
      {/each}
    </div>
  </section>

  {#if $games.length === 0}
    <div class="empty-state">
      <p class="text-mono text-secondary">No games in your library yet.</p>
      <p class="text-mono text-tertiary text-sm">
        Click the + button to add games or connect your Steam account.
      </p>
    </div>
  {/if}
</div>

<style>
  .home-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
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
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }

  .empty-state p {
    margin: var(--space-2) 0;
  }
</style>
