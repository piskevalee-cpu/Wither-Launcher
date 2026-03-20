<script lang="ts">
  import { page } from '$app/stores';

  let { href, icon, label, count }: { href: string; icon: string; label: string; count?: number } = $props();
  let isActive = $derived($page.url.pathname === href);
</script>

<a href={href} class="nav-item" class:active={isActive}>
  <span class="nav-icon">{@html icon}</span>
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
    color: var(--text-secondary);
    transition: all 0.15s ease;
    cursor: pointer;
    text-decoration: none;
  }

  .nav-item:hover {
    background: var(--bg-s2);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--bg-s3);
    color: var(--text-primary);
  }

  .nav-icon {
    font-size: var(--text-lg);
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .nav-label {
    flex: 1;
  }

  .nav-count {
    background: var(--bg-s1);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--text-tertiary);
  }

  .nav-item.active .nav-count {
    background: var(--text-primary);
    color: var(--bg-root);
  }
</style>
