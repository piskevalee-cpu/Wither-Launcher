<script lang="ts">
  import { games } from '$lib/stores';

  let totalPlaytime = $derived(
    Math.round($games.reduce((sum, g) => sum + g.wither_playtime_s, 0) / 3600)
  );
  let totalGames = $derived($games.length);
  let mostPlayed = $derived(
    $games.sort((a, b) => b.wither_playtime_s - a.wither_playtime_s)[0]?.name || '—'
  );
</script>

<footer class="status-bar">
  <div class="status-item text-mono text-xs text-secondary">
    <span class="text-primary">{totalGames}</span> games
  </div>
  <div class="status-item text-mono text-xs text-secondary">
    <span class="text-primary">{totalPlaytime}</span> hours played
  </div>
  <div class="status-item text-mono text-xs text-secondary">
    Most played: <span class="text-accent">{mostPlayed}</span>
  </div>
  <div class="status-item text-mono text-xs text-success">
    ● Synced
  </div>
</footer>

<style>
  .status-bar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--space-6);
    height: 32px;
    padding: 0 var(--space-4);
    background: var(--color-bg-1);
    border-top: 1px solid var(--color-border-1);
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
</style>
