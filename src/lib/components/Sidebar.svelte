<script lang="ts">
  import NavItem from './NavItem.svelte';
  import SyncStatusDot from './SyncStatusDot.svelte';
  import { games, ui } from '$lib/stores';
  import { getAllGames, addCustomGame, syncSteam } from '$lib/api';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  let totalGames = $derived($games.length);
  let steamUser = $state<{ personaname: string; avatar: string } | null>(null);

  // Load Steam user info
  $effect(() => {
    async function loadSteamUser() {
      try {
        const user = await invoke<any>('get_steam_user');
        if (user) {
          steamUser = user;
        }
      } catch (error) {
        console.error('Failed to load Steam user:', error);
      }
    }
    loadSteamUser();
  });

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

      console.log('Selected file:', filePath);

      if (filePath) {
        ui.setLoading(true);
        const game = await addCustomGame({
          executable_path: filePath as string
        });
        console.log('Added game:', game);
        games.addGame(game);
        ui.setLoading(false);
      }
    } catch (error) {
      console.error('Failed to add game:', error);
      alert('Failed to add game: ' + error);
      ui.setLoading(false);
    }
  }

  async function handleSyncSteam() {
    try {
      ui.setLoading(true);
      const result = await syncSteam();
      const allGames = await getAllGames();
      games.set(allGames);
      ui.setLoading(false);
      alert(`Steam sync complete!\nAdded: ${result.added}\nUpdated: ${result.updated}\nRemoved: ${result.removed}`);
    } catch (error) {
      console.error('Failed to sync Steam:', error);
      alert('Sync failed: ' + error);
      ui.setLoading(false);
    }
  }
</script>

<aside class="sidebar" class:collapsed={!$ui.sidebarOpen}>
  <button class="sb-toggle" onclick={() => ui.toggleSidebar()} title={$ui.sidebarOpen ? 'Collapse sidebar' : 'Expand sidebar'}>
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.5">
      <path d="M2 1l6 4-6 4" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
  </button>

  <div class="sb-header">
    <span class="sb-logo text-mono text-logo font-semibold">Wither</span>
  </div>

  <nav class="sb-nav">
    <NavItem href="/" icon="◈" label="Home" count={totalGames} />
    <NavItem href="/library" icon="▤" label="Library" />
    <NavItem href="/recent" icon="◷" label="Recent" />
    <NavItem href="/stats" icon="📊" label="Stats" />
  </nav>

  <div class="sb-divider"></div>

  <nav class="sb-nav">
    <NavItem href="/store" icon="🛒" label="Steam Store" />
  </nav>

  <div class="sb-divider"></div>

  <div class="sb-actions">
    <button class="sb-add-btn" onclick={handleAddGame}>
      + Add Game
    </button>
    <button class="sb-sync-btn" onclick={handleSyncSteam}>
      ↻ Sync Steam
    </button>
  </div>

  <div class="sb-divider"></div>

  <div class="sb-platforms">
    <p class="sb-platform-label text-mono text-xs text-tertiary">Platforms</p>
    <div class="sb-platform-item">
      <span class="sb-status-dot success"></span>
      <span class="text-sans text-sm text-secondary">Steam</span>
    </div>
  </div>

  <div class="sb-footer">
    <NavItem href="/settings" icon="⚙" label="Settings" />
    
    {#if steamUser}
      <div class="sb-user-row">
        {#if steamUser.avatar}
          <img src={steamUser.avatar} alt={steamUser.personaname} class="sb-user-avatar" />
        {/if}
        <span class="sb-username text-sans text-sm text-secondary">{steamUser.personaname}</span>
        <SyncStatusDot />
      </div>
    {:else}
      <div class="sb-user-row">
        <span class="sb-username text-sans text-sm text-tertiary">Not logged in</span>
        <SyncStatusDot />
      </div>
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    grid-row: 1 / -1;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border-1);
    display: flex;
    flex-direction: column;
    padding: var(--space-4);
    gap: var(--space-3);
    width: 220px;
    position: relative;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: 54px;
    padding: var(--space-4) var(--space-2);
  }

  .sb-toggle {
    position: absolute;
    top: var(--space-4);
    right: var(--space-2);
    width: 20px;
    height: 20px;
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
    transition: all 0.15s ease;
  }

  .sb-toggle:hover {
    color: var(--text-primary);
  }

  .sidebar.collapsed .sb-toggle {
    transform: scaleX(-1);
  }

  .sb-header {
    padding: var(--space-2) 0;
    margin-top: var(--space-6);
  }

  .sb-logo {
    color: var(--text-primary);
    letter-spacing: 0.5px;
  }

  .sidebar.collapsed .sb-logo {
    opacity: 0;
    pointer-events: none;
  }

  .sb-nav {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .sb-divider {
    height: 1px;
    background: var(--border-1);
    margin: var(--space-2) 0;
  }

  .sb-actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sb-add-btn,
  .sb-sync-btn {
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    color: var(--text-primary);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-pill);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .sb-add-btn:hover,
  .sb-sync-btn:hover {
    background: var(--bg-s2);
    border-color: var(--border-2);
  }

  .sidebar.collapsed .sb-add-btn,
  .sidebar.collapsed .sb-sync-btn {
    opacity: 0;
    pointer-events: none;
  }

  .sb-platforms {
    padding: var(--space-3) 0;
  }

  .sb-platform-label {
    margin-bottom: var(--space-2);
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .sidebar.collapsed .sb-platform-label {
    opacity: 0;
  }

  .sb-platform-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
  }

  .sb-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--status-synced);
  }

  .sb-footer {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sb-user-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
  }

  .sb-user-avatar {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    object-fit: cover;
  }

  .sb-username {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sidebar.collapsed .sb-username {
    opacity: 0;
    pointer-events: none;
  }

  .sidebar.collapsed .sb-user-avatar {
    margin: 0 auto;
  }
</style>
