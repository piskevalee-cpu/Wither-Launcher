<script lang="ts">
  import { syncSteam, steamLogin, steamLogout, getSteamUser, resetSteamGames, getAllGames, clearRemovedSteamGames } from '$lib/api';
  import { ui } from '$lib/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { SteamUser } from '$lib/types';

  let isSyncing = $state(false);
  let lastSyncTime = $state<string | null>(null);
  let closeToTray = $state(true);
  let isLoading = $state(true);
  let steamUser = $state<SteamUser | null>(null);
  let steamApiKey = $state('');
  let steamGridDbKey = $state('');

  // Load settings and Steam user on mount
  $effect(() => {
    async function loadSettings() {
      try {
        const [closeToTrayValue, apiKey, gridDbKey] = await Promise.all([
          invoke<string>('get_setting', { key: 'close_to_tray' }),
          invoke<string>('get_setting', { key: 'steam_api_key' }),
          invoke<string>('get_setting', { key: 'steamgriddb_api_key' }),
        ]);
        closeToTray = closeToTrayValue !== 'false';
        steamApiKey = apiKey;
        steamGridDbKey = gridDbKey;
      } catch (error) {
        console.error('Failed to load settings:', error);
      } finally {
        isLoading = false;
      }
    }
    
    async function loadSteamUser() {
      try {
        steamUser = await getSteamUser();
      } catch (error) {
        console.error('Failed to load Steam user:', error);
      }
    }
    
    loadSettings();
    loadSteamUser();
  });

  async function handleSync() {
    if (isSyncing) return;
    isSyncing = true;
    ui.setLoading(true);
    console.log('Starting Steam sync...');
    try {
      const result = await syncSteam();
      console.log('Sync result:', result);
      lastSyncTime = new Date(result.synced_at * 1000).toLocaleString();
      
      let message = `Sync complete!\nAdded: ${result.added}\nUpdated: ${result.updated}\nRemoved: ${result.removed}`;
      if (result.errors.length > 0) {
        message += `\n\nErrors:\n${result.errors.join('\n')}`;
      }
      alert(message);
      
      // Force refresh the games store
      if (result.added > 0 || result.updated > 0) {
        window.location.reload();
      }
    } catch (error) {
      console.error('Sync failed:', error);
      alert('Sync failed: ' + error);
    } finally {
      isSyncing = false;
      ui.setLoading(false);
    }
  }

  async function handleSteamLogin() {
    try {
      steamUser = await steamLogin();
      alert(`Logged in as ${steamUser.personaname}!`);
    } catch (error) {
      console.error('Steam login failed:', error);
      alert('Steam login failed: ' + error);
    }
  }

  async function handleSteamLogout() {
    try {
      await steamLogout();
      steamUser = null;
      alert('Logged out from Steam');
    } catch (error) {
      console.error('Steam logout failed:', error);
    }
  }

  async function handleDebugSteam() {
    try {
      const debug = await invoke('debug_steam_paths');
      console.log('Steam Debug Info:', debug);
      
      // Also sync and show detailed info
      const result = await syncSteam();
      const allGames = await getAllGames();
      
      const steamGames = allGames.filter((g: any) => g.source === 'steam');
      const installedGames = steamGames.filter((g: any) => g.is_installed);
      const uninstalledGames = steamGames.filter((g: any) => !g.is_installed);
      
      // Read actual ACF file content for debugging
      let acfContent = '';
      try {
        acfContent = await readAcfFile(227300); // Euro Truck Simulator 2
      } catch (e) {
        acfContent = 'Could not read ACF file: ' + e;
      }
      
      let message = `Steam Debug:\n\n`;
      message += `Steam Root: ${debug.steam_root || 'NOT FOUND'}\n\n`;
      message += `Library Folders:\n${JSON.stringify(debug.library_folders, null, 2)}\n\n`;
      message += `ACF Files Found: ${debug.acf_files?.length || 0}\n\n`;
      message += `First 5 ACF files:\n${(debug.acf_files || []).slice(0, 5).join('\n')}\n\n`;
      message += `─────────────────────────\n`;
      message += `Sync Result:\nAdded: ${result.added}, Updated: ${result.updated}, Removed: ${result.removed}\n\n`;
      message += `─────────────────────────\n`;
      message += `Steam Games in DB: ${steamGames.length}\n`;
      message += `  - Installed: ${installedGames.length}\n`;
      message += `  - Uninstalled: ${uninstalledGames.length}\n\n`;
      
      if (installedGames.length > 0) {
        message += `INSTALLED GAMES:\n`;
        installedGames.forEach((g: any) => {
          message += `  ✅ ${g.name} (AppID: ${g.steam_app_id})\n`;
        });
      }
      
      if (uninstalledGames.length > 0) {
        message += `\nUNINSTALLED GAMES:\n`;
        uninstalledGames.forEach((g: any) => {
          message += `  ❌ ${g.name} (AppID: ${g.steam_app_id})\n`;
        });
      }
      
      message += `\n─────────────────────────\n`;
      message += `ACF FILE CONTENT (AppID 227300):\n\n${acfContent}\n`;
      
      alert(message);
    } catch (error) {
      console.error('Debug failed:', error);
      alert('Debug failed: ' + error);
    }
  }

  async function handleCheckSessions() {
    try {
      const sessions = await invoke<any[]>('get_active_sessions');
      let message = 'Active Sessions:\n\n';
      if (sessions.length === 0) {
        message += 'No active sessions\n';
      } else {
        sessions.forEach(s => {
          message += `Game: ${s.name}\n`;
          message += `  Session: ${s.id}\n`;
          message += `  Started: ${new Date(s.started_at * 1000).toLocaleString()}\n`;
          message += `  Duration: ${Math.floor((Date.now()/1000 - s.started_at) / 60)} minutes\n\n`;
        });
      }
      alert(message);
    } catch (error) {
      alert('Session check failed: ' + error);
    }
  }

  async function handleResetSteamGames() {
    if (!confirm('This will delete ALL Steam games from your library and re-sync them with proper names.\n\nContinue?')) {
      return;
    }
    
    try {
      const count = await resetSteamGames();
      console.log(`Deleted ${count} Steam games`);
      
      // Auto-sync after reset
      alert(`Deleted ${count} Steam games.\n\nNow syncing with proper names...`);
      isSyncing = true;
      ui.setLoading(true);
      
      try {
        const result = await syncSteam();
        alert(`Sync complete!\nAdded: ${result.added}\nUpdated: ${result.updated}\nRemoved: ${result.removed}`);
        window.location.reload();
      } catch (error) {
        console.error('Sync failed:', error);
        alert('Sync failed: ' + error);
      } finally {
        isSyncing = false;
        ui.setLoading(false);
      }
    } catch (error) {
      console.error('Reset failed:', error);
      alert('Reset failed: ' + error);
    }
  }

  async function handleClearRemovedGames() {
    try {
      const count = await clearRemovedSteamGames();
      alert(`Cleared ${count} removed games from tracking.\n\nNow Steam games can be re-added on next sync!`);
      window.location.reload();
    } catch (error) {
      console.error('Clear failed:', error);
      alert('Clear failed: ' + error);
    }
  }

  async function handleSteamApiKeyChange(event: Event) {
    const target = event.target as HTMLInputElement;
    steamApiKey = target.value;
    
    try {
      await invoke('set_setting', {
        key: 'steam_api_key',
        value: steamApiKey
      });
    } catch (error) {
      console.error('Failed to save API key:', error);
    }
  }

  async function handleSteamGridDbKeyChange(event: Event) {
    const target = event.target as HTMLInputElement;
    steamGridDbKey = target.value;
    
    try {
      await invoke('set_setting', {
        key: 'steamgriddb_api_key',
        value: steamGridDbKey
      });
    } catch (error) {
      console.error('Failed to save SteamGridDB key:', error);
    }
  }

  async function handleCloseToTrayChange(event: Event) {
    const target = event.target as HTMLInputElement;
    closeToTray = target.checked;
    
    try {
      await invoke('set_setting', { 
        key: 'close_to_tray', 
        value: closeToTray ? 'true' : 'false' 
      });
    } catch (error) {
      console.error('Failed to save setting:', error);
      // Revert on error
      closeToTray = !closeToTray;
      target.checked = closeToTray;
    }
  }

  async function handleCloseApp() {
    const appWindow = getCurrentWindow();
    if (closeToTray) {
      await appWindow.hide();
    } else {
      await appWindow.close();
    }
  }
</script>

<div class="settings-page">
  <h1 class="page-title text-mono text-xl text-primary">Settings</h1>

  <section class="settings-section">
    <h2 class="section-title text-mono text-base text-primary">Steam Account</h2>

    <div class="setting-item" style="margin-bottom: 10px; gap: 10px; flex-wrap: wrap;">
      <button onclick={handleDebugSteam} style="background: #666; padding: 8px 16px;">
        🔍 Debug
      </button>
      <button onclick={handleCheckSessions} style="background: #2196F3; padding: 8px 16px;">
        📊 Check Active Sessions
      </button>
      <button onclick={handleClearRemovedGames} style="background: #ff9800; padding: 8px 16px;">
        🗑 Clear Removed Games
      </button>
      <button onclick={handleResetSteamGames} style="background: #d62828; padding: 8px 16px;">
        🔄 Reset & Re-Sync
      </button>
    </div>

    {#if steamUser}
      <!-- Logged in -->
      <div class="steam-profile">
        <div class="steam-avatar-wrapper">
          {#if steamUser.avatar && steamUser.avatar.startsWith('http')}
            <img 
              src={steamUser.avatar} 
              alt={steamUser.personaname} 
              class="steam-avatar"
              onload={console.log('Avatar loaded:', steamUser.avatar)}
              onerror={console.error('Avatar failed to load:', steamUser.avatar)}
            />
          {:else}
            <div class="avatar-fallback">
              {steamUser.personaname.charAt(0).toUpperCase()}
            </div>
          {/if}
        </div>
        <div class="steam-info">
          <p class="steam-name text-mono text-base text-primary">{steamUser.personaname}</p>
          <p class="steam-id text-sans text-sm text-secondary">ID: {steamUser.steamid}</p>
          <a href={steamUser.profileurl} target="_blank" class="steam-profile-link text-sans text-sm text-accent">
            View Steam Profile →
          </a>
          {#if !steamUser.avatar || !steamUser.avatar.startsWith('http')}
            <p class="avatar-note text-sans text-xs text-tertiary">
              💡 Add your Steam API Key above to fetch your avatar
            </p>
          {/if}
        </div>
        <button onclick={handleSteamLogout} class="logout-button">
          Logout
        </button>
      </div>
    {:else}
      <!-- Not logged in -->
      <div class="setting-item">
        <div class="setting-info">
          <p class="setting-label text-sans text-md text-primary">Connect Steam Account</p>
          <p class="setting-description text-sans text-sm text-secondary">
            Log in with Steam to sync your library, playtime, and achievements.
          </p>
        </div>
        <button onclick={handleSteamLogin} class="steam-login-button">
          🎮 Login with Steam
        </button>
      </div>
    {/if}
  </section>

  <section class="settings-section">
    <h2 class="section-title text-mono text-base text-primary">Steam Integration</h2>
    
    <div class="setting-item">
      <div class="setting-info">
        <p class="setting-label text-sans text-md text-primary">Sync Steam Library</p>
        <p class="setting-description text-sans text-sm text-secondary">
          Scan your Steam installation for installed games and import them into Wither.
        </p>
      </div>
      <button onclick={handleSync} disabled={isSyncing}>
        {#if isSyncing}
          Syncing...
        {:else}
          Sync Now
        {/if}
      </button>
    </div>

    {#if lastSyncTime}
      <p class="sync-status text-mono text-xs text-tertiary">
        Last sync: {lastSyncTime}
      </p>
    {/if}
  </section>

  <section class="settings-section">
    <h2 class="section-title text-mono text-base text-primary">API Keys</h2>
    
    <div class="setting-item">
      <div class="setting-info">
        <p class="setting-label text-sans text-md text-primary">Steam Web API Key</p>
        <p class="setting-description text-sans text-sm text-secondary">
          Required for fetching game metadata (covers, genres, etc.).
          <a href="https://steamcommunity.com/dev/apikey" target="_blank">Get your key here</a>.
        </p>
      </div>
      <input 
        type="password" 
        value={steamApiKey}
        oninput={handleSteamApiKeyChange}
        placeholder="Enter API key" 
        class="api-input" 
      />
    </div>

    <div class="setting-item">
      <div class="setting-info">
        <p class="setting-label text-sans text-md text-primary">SteamGridDB API Key</p>
        <p class="setting-description text-sans text-sm text-secondary">
          Required for fetching cover art for custom games.
          <a href="https://www.steamgriddb.com/api/v2" target="_blank">Get your key here</a>.
        </p>
      </div>
      <input 
        type="password"
        value={steamGridDbKey}
        oninput={handleSteamGridDbKeyChange}
        placeholder="Enter API key" 
        class="api-input" 
      />
    </div>
  </section>

  <section class="settings-section">
    <h2 class="section-title text-mono text-base text-primary">Behavior</h2>
    
    <div class="setting-item">
      <div class="setting-info">
        <p class="setting-label text-sans text-md text-primary">Close to System Tray</p>
        <p class="setting-description text-sans text-sm text-secondary">
          When enabled, clicking the X button minimizes to tray. When disabled, the app closes completely.
        </p>
      </div>
      <label class="toggle">
        <input 
          type="checkbox" 
          checked={closeToTray} 
          onchange={handleCloseToTrayChange}
          disabled={isLoading}
        />
        <span class="toggle-slider"></span>
      </label>
    </div>
  </section>

  <section class="settings-section">
    <h2 class="section-title text-mono text-base text-primary">About</h2>
    
    <div class="about-info">
      <p class="text-mono text-base text-primary">Wither Launcher</p>
      <p class="text-sans text-sm text-secondary">Version 0.1.0</p>
      <p class="text-sans text-sm text-tertiary">
        A cross-platform game launcher with minimal resource footprint.
      </p>
    </div>
  </section>
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 700px;
  }

  .page-title {
    font-weight: 600;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding-bottom: var(--space-4);
    border-bottom: 1px solid var(--color-border-1);
  }

  .section-title {
    font-weight: 500;
  }

  .setting-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .setting-info {
    flex: 1;
  }

  .setting-label {
    margin-bottom: var(--space-1);
  }

  .setting-description {
    max-width: 500px;
  }

  .setting-description a {
    color: var(--color-accent);
  }

  .api-input {
    width: 300px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-border-1);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-md);
    color: var(--color-text-primary);
  }

  .api-input:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .sync-status {
    margin-top: var(--space-2);
  }

  .about-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* Modern Settings Buttons (matching game card buttons) */
  .settings-section button {
    background: rgba(255, 255, 255, 0.15);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: var(--text-primary);
    padding: 8px 16px;
    border-radius: var(--radius-pill);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .settings-section button:hover {
    background: rgba(255, 255, 255, 0.25);
    border-color: rgba(255, 255, 255, 0.4);
    transform: scale(1.05);
  }

  /* Steam Login Section */
  .steam-profile {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    border: 1px solid var(--color-border-1);
  }

  .steam-avatar-wrapper {
    flex-shrink: 0;
  }

  .steam-avatar {
    width: 64px;
    height: 64px;
    border-radius: var(--radius-md);
    border: 2px solid var(--color-accent);
    object-fit: cover;
  }

  .avatar-fallback {
    width: 64px;
    height: 64px;
    border-radius: var(--radius-md);
    border: 2px solid var(--color-accent);
    background: var(--color-accent-dim);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-size: 24px;
    font-weight: 600;
    color: var(--color-accent);
  }

  .steam-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .steam-name {
    font-weight: 500;
  }

  .steam-profile-link {
    color: var(--color-accent);
    text-decoration: none;
  }

  .steam-profile-link:hover {
    text-decoration: underline;
  }

  .avatar-note {
    margin-top: var(--space-2);
    padding: var(--space-2);
    background: var(--color-bg-3);
    border-radius: var(--radius-sm);
  }

  .logout-button {
    background: var(--color-bg-3);
    border: 1px solid var(--color-border-1);
    color: var(--color-text-primary);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    cursor: pointer;
  }

  .logout-button:hover {
    background: var(--color-accent);
    border-color: var(--color-accent);
  }

  .steam-login-button {
    background: #171a21;
    border: none;
    color: #ffffff;
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-md);
    font-family: var(--font-mono);
    font-size: var(--text-base);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .steam-login-button:hover {
    background: #2a475e;
    transform: translateY(-2px);
  }

  .toggle {
    position: relative;
    display: inline-block;
    width: 50px;
    height: 26px;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    inset: 0;
    background: var(--bg-s2);
    border: 1px solid var(--border-1);
    border-radius: 34px;
    transition: all 0.2s ease;
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 3px;
    bottom: 3px;
    background: var(--text-tertiary);
    border-radius: 50%;
    transition: all 0.2s ease;
  }

  input:checked + .toggle-slider {
    background: var(--text-primary);
    border-color: var(--text-primary);
  }

  input:checked + .toggle-slider:before {
    transform: translateX(24px);
    background: #000;
  }

  input:disabled + .toggle-slider {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
