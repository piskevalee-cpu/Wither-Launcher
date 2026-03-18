<script lang="ts">
  import { games } from '$lib/stores';

  let totalPlaytime = $derived(
    Math.round($games.reduce((sum, g) => sum + g.wither_playtime_s, 0) / 3600)
  );
  let totalGames = $derived($games.length);
  let totalSessions = $derived($games.reduce((sum, g) => sum + g.session_count, 0));
  
  let topGames = $derived(
    [...$games]
      .sort((a, b) => b.wither_playtime_s - a.wither_playtime_s)
      .slice(0, 10)
      .map((g, i) => ({
        rank: i + 1,
        name: g.name,
        hours: Math.round(g.wither_playtime_s / 3600),
      }))
  );
</script>

<div class="stats-page">
  <h1 class="page-title text-mono text-xl text-primary">Statistics</h1>
  
  <div class="stats-grid">
    <div class="stat-card">
      <p class="stat-value text-mono text-xl text-primary">{totalGames}</p>
      <p class="stat-label text-sans text-sm text-secondary">Total Games</p>
    </div>
    <div class="stat-card">
      <p class="stat-value text-mono text-xl text-primary">{totalPlaytime}</p>
      <p class="stat-label text-sans text-sm text-secondary">Hours Played</p>
    </div>
    <div class="stat-card">
      <p class="stat-value text-mono text-xl text-primary">{totalSessions}</p>
      <p class="stat-label text-sans text-sm text-secondary">Sessions</p>
    </div>
  </div>

  <section class="top-games">
    <h2 class="section-title text-mono text-base text-primary">Top Games by Playtime</h2>
    {#if topGames.length > 0}
      <div class="top-list">
        {#each topGames as game (game.rank)}
          <div class="top-item">
            <span class="rank text-mono text-sm text-tertiary">#{game.rank}</span>
            <span class="name text-sans text-md text-primary">{game.name}</span>
            <span class="hours text-mono text-sm text-accent">{game.hours}h</span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-mono text-secondary">No playtime data yet.</p>
    {/if}
  </section>
</div>

<style>
  .stats-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  .page-title {
    font-weight: 600;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-4);
  }

  .stat-card {
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .top-games {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .section-title {
    font-weight: 500;
  }

  .top-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
  }

  .top-item {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--color-border-1);
  }

  .top-item:last-child {
    border-bottom: none;
  }

  .rank {
    width: 30px;
  }

  .name {
    flex: 1;
  }

  .hours {
    font-weight: 500;
  }
</style>
