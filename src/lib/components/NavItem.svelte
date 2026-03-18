<script lang="ts">
  import { page } from '$app/stores';

  let { href, icon, label, count }: { href: string; icon: string; label: string; count?: number } = $props();
  let isActive = $derived($page.url.pathname === href);
</script>

<a href={href} class="nav-item" class:active={isActive}>
  <span class="nav-icon">{icon}</span>
  <span class="nav-label text-sans text-md">{label}</span>
  {#if count !== undefined && count > 0}
    <span class="nav-count text-mono text-xs">{count}</span>
  {/if}
</a>

<style>
  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: var(--color-text-secondary);
    transition: all 0.15s ease;
    cursor: pointer;
  }

  .nav-item:hover {
    background: var(--color-bg-2);
    color: var(--color-text-primary);
  }

  .nav-item.active {
    background: var(--color-bg-3);
    color: var(--color-text-primary);
  }

  .nav-icon {
    font-size: var(--text-lg);
    width: 20px;
    text-align: center;
  }

  .nav-label {
    flex: 1;
  }

  .nav-count {
    background: var(--color-bg-2);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-tertiary);
  }

  .nav-item.active .nav-count {
    background: var(--color-accent);
    color: var(--color-text-primary);
  }
</style>
