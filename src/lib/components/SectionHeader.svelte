<script lang="ts">
  let { title, viewAll }: { title: string; viewAll?: string } = $props();
</script>

<div class="section-header">
  <h2 class="section-title text-mono text-base text-primary">{title}</h2>
  {#if viewAll}
    <a href={viewAll} class="view-all text-sans text-sm text-secondary">
      View all →
    </a>
  {/if}
</div>

<style>
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .section-title {
    font-weight: 500;
    letter-spacing: 0.5px;
  }

  .view-all {
    transition: color 0.15s ease;
  }

  .view-all:hover {
    color: var(--color-text-primary);
  }
</style>
