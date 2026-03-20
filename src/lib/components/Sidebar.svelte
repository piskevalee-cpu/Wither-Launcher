<script lang="ts">
  import NavItem from './NavItem.svelte';
  import SyncStatusDot from './SyncStatusDot.svelte';
  import { games, refreshAfterSync, ui, addCustomGame } from '$lib/stores';
  import { syncSteam } from '$lib/api';
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
      // Determine file filter based on platform
      const filePath = await open({
        title: 'Select Game Executable',
        multiple: false,
        filters: [
          {
            name: 'Game Executables',
            extensions: ['exe', 'sh', 'AppImage', 'x86_64', 'x86', '']
          },
          {
            name: 'All Files',
            extensions: ['*']
          }
        ]
      });

      console.log('Selected file:', filePath);

      if (filePath) {
        ui.setLoading(true);
        try {
          await addCustomGame({ executable_path: filePath as string });
          ui.setLoading(false);
        } catch (error) {
          console.error('Failed to add game:', error);
          alert('Failed to add game: ' + error);
          ui.setLoading(false);
        }
      }
    } catch (error) {
      console.error('Failed to open file dialog:', error);
      alert('Failed to open file dialog: ' + error);
      ui.setLoading(false);
    }
  }

  async function handleSyncSteam() {
    try {
      ui.setLoading(true);
      const result = await syncSteam();
      await refreshAfterSync();
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
    <span class="sb-logo text-sans font-medium">Wither Launcher</span>
  </div>

  <nav class="sb-nav">
    <NavItem href="/" icon={`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path><polyline points="9 22 9 12 15 12 15 22"></polyline></svg>`} label="Home" count={totalGames} />
    <NavItem href="/library" icon={`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7"></rect><rect x="14" y="3" width="7" height="7"></rect><rect x="14" y="14" width="7" height="7"></rect><rect x="3" y="14" width="7" height="7"></rect></svg>`} label="Library" />
    <NavItem href="/recent" icon={`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>`} label="Recent" />
    <NavItem href="/stats" icon={`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="20" x2="12" y2="10"></line><line x1="18" y1="20" x2="18" y2="4"></line><line x1="6" y1="20" x2="6" y2="16"></line></svg>`} label="Stats" />
  </nav>

  <div class="sb-divider"></div>

  <nav class="sb-nav">
    <NavItem href="/store" icon={`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M6 2 3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4Z"></path><line x1="3" y1="6" x2="21" y2="6"></line><path d="M16 10a4 4 0 0 1-8 0"></path></svg>`} label="Steam Store" />
  </nav>

  <div class="sb-divider"></div>

  <div class="sb-actions">
    <button class="sb-btn" onclick={handleAddGame}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
      <span>Add Game</span>
    </button>
    <button class="sb-btn" onclick={handleSyncSteam}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><polyline points="1 20 1 14 7 14"></polyline><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg>
      <span>Sync Steam</span>
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
    <NavItem href="/settings" icon={`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`} label="Settings" />
    
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
    min-width: 220px;
    position: relative;
    overflow: hidden;
    transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1),
               min-width 0.25s cubic-bezier(0.4, 0, 0.2, 1),
               padding 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .sidebar.collapsed {
    width: 54px;
    min-width: 54px;
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
    right: 50%;
    translate: 50% 0;
  }

  .sb-header {
    height: 54px;
    display: flex;
    align-items: center;
    margin-top: calc(-1 * var(--space-4));
    margin-bottom: var(--space-2);
    overflow: hidden;
    white-space: nowrap;
  }

  .sb-logo {
    color: var(--text-primary);
    letter-spacing: 0.5px;
    transition: opacity 0.15s ease;
  }

  .sidebar.collapsed .sb-logo {
    opacity: 0;
    pointer-events: none;
  }

  .sidebar.collapsed .sb-header {
    height: 0;
    padding: 0;
    margin: 0;
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
    overflow: hidden;
    transition: height 0.25s ease, opacity 0.15s ease;
  }

  .sb-btn {
    background: var(--bg-s1);
    border: 1px solid var(--border-1);
    color: var(--text-primary);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-pill);
    font-family: var(--font-sans);
    font-size: var(--text-base);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .sb-btn:hover {
    background: var(--bg-s2);
    border-color: var(--border-2);
  }

  .sidebar.collapsed .sb-actions {
    height: 0;
    opacity: 0;
    pointer-events: none;
    margin: 0;
    gap: 0;
    overflow: hidden;
  }

  .sidebar.collapsed .sb-btn {
    opacity: 0;
    pointer-events: none;
    height: 0;
    padding: 0;
    margin: 0;
    border: none;
  }

  .sb-platforms {
    padding: var(--space-3) 0;
    overflow: hidden;
    transition: height 0.25s ease, opacity 0.15s ease, padding 0.25s ease;
  }

  .sidebar.collapsed .sb-platforms {
    height: 0;
    opacity: 0;
    padding: 0;
    pointer-events: none;
  }

  .sb-platform-label {
    margin-bottom: var(--space-2);
    text-transform: uppercase;
    letter-spacing: 1px;
    white-space: nowrap;
  }

  .sidebar.collapsed .sb-platform-label {
    opacity: 0;
  }

  .sb-platform-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    white-space: nowrap;
  }

  .sidebar.collapsed .sb-platform-item span:not(.sb-status-dot) {
    display: none;
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
    overflow: hidden;
    white-space: nowrap;
  }

  .sb-user-avatar {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .sb-username {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: opacity 0.15s ease;
  }

  .sidebar.collapsed .sb-username {
    opacity: 0;
    width: 0;
    pointer-events: none;
  }

  .sidebar.collapsed .sb-user-avatar {
    margin: 0 auto;
  }

  /* Hide dividers in collapsed state to keep it clean */
  .sidebar.collapsed .sb-divider {
    margin: var(--space-1) 0;
  }

  /* Hide NavItem labels and count badges when collapsed (cross-component via :global) */
  .sidebar.collapsed :global(.nav-label) {
    display: none;
  }

  .sidebar.collapsed :global(.nav-count) {
    display: none;
  }

  .sidebar.collapsed :global(.nav-item) {
    justify-content: center;
    padding: var(--space-2);
  }

  .sidebar.collapsed :global(.nav-icon) {
    margin: 0;
  }
</style>
