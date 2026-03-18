<script lang="ts">
  import { syncState, lastSyncedAt } from '$lib/stores/syncStore';

  let state = $derived($syncState);
  let lastSync = $derived($lastSyncedAt);

  function getDotClass() {
    switch (state) {
      case 'syncing': return 'syncing';
      case 'synced': return 'synced';
      case 'error': return 'error';
      default: return 'disconnected';
    }
  }

  function getLabel() {
    switch (state) {
      case 'syncing': return 'Syncing...';
      case 'synced': return lastSync ? 'Steam synced' : 'Steam synced';
      case 'error': return 'Sync failed';
      default: return 'Not connected';
    }
  }

  function showSpinner() {
    return state === 'syncing';
  }
</script>

<div class="sync-status">
  <span class="sync-dot" class:syncing={state === 'syncing'} class:synced={state === 'synced'} class:error={state === 'error'} class:disconnected={state === 'disconnected'}></span>
  {#if showSpinner()}
    <svg class="sync-spinner" viewBox="0 0 16 16" fill="none">
      <path d="M8 1a7 7 0 1 0 7 7" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>
  {/if}
  <span class="sync-label text-sans text-xs text-tertiary">{getLabel()}</span>
</div>

<style>
  .sync-status {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .sync-dot.syncing {
    background: var(--status-syncing);
    animation: pulse 1s ease-in-out infinite;
  }

  .sync-dot.synced {
    background: var(--status-synced);
  }

  .sync-dot.error {
    background: var(--status-error);
  }

  .sync-dot.disconnected {
    background: var(--status-disconnected);
  }

  .sync-spinner {
    width: 12px;
    height: 12px;
    color: var(--status-syncing);
    animation: spin 1s linear infinite;
  }

  .sync-label {
    white-space: nowrap;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
