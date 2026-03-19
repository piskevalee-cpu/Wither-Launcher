<script lang="ts">
  import { syncSteam, steamLogin, steamLogout, getSteamUser, resetSteamGames, getAllGames, clearRemovedSteamGames, readAcfFile, getProtonVersions, type ProtonVersion, getProtonGeReleases, downloadProtonGe, type ProtonGeRelease } from '$lib/api';
  import type { SteamUser } from '$lib/types';
  import { ui } from '$lib/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';

  let isSyncing = $state(false);
  let lastSyncTime = $state<string | null>(null);
  let closeToTray = $state(true);
  let isLoading = $state(true);
  let steamUser = $state<SteamUser | null>(null);
  let steamApiKey = $state('');
  let steamGridDbKey = $state('');
  let protonVersions = $state<ProtonVersion[]>([]);
  let selectedProtonPath = $state('');
  let isLinux = $state(false);
  
  // GE-Proton download state
  let geReleases = $state<ProtonGeRelease[]>([]);
  let isFetchingReleases = $state(false);
  let downloadingTag = $state<string | null>(null);
  let downloadProgress = $state<{ stage: string; percent: number } | null>(null);

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
        
        // If no user in DB, try auto-detecting from local Steam config
        if (!steamUser || !steamUser.steamid) {
          const detected = await invoke<SteamUser | null>('auto_detect_steam_user');
          if (detected) {
            steamUser = detected;
          }
        }
      } catch (error) {
        console.error('Failed to load Steam user:', error);
      }
    }
    
    loadSettings();
    loadSteamUser();
    loadProtonVersions();
    
    // Listen for download progress events
    const unlisten = listen<{ stage: string; tag: string; percent: number }>('proton_download_progress', (event) => {
      downloadProgress = { stage: event.payload.stage, percent: event.payload.percent };
      if (event.payload.stage === 'done') {
        downloadingTag = null;
        downloadProgress = null;
        // Refresh versions after install
        loadProtonVersions();
        fetchGeReleases();
      }
    });
    
    return () => { unlisten.then(fn => fn()); };
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

  async function handleAutoDetect() {
    try {
      const detected = await invoke<SteamUser | null>('auto_detect_steam_user');
      if (detected) {
        steamUser = detected;
        alert(`Auto-detected Steam user: ${detected.personaname}`);
      } else {
        alert('Could not auto-detect Steam user. Make sure Steam is installed and you are logged in.');
      }
    } catch (error) {
      console.error('Auto-detect failed:', error);
      alert('Auto-detect failed: ' + error);
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
      const debug = await invoke('debug_steam_paths') as any;
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

  async function loadProtonVersions() {
    try {
      const versions = await getProtonVersions();
      protonVersions = versions;
      isLinux = versions.length > 0 || navigator.platform.toLowerCase().includes('linux');
      
      // Load saved proton path
      const savedPath = await invoke<string>('get_setting', { key: 'proton_path' });
      selectedProtonPath = savedPath || '';
    } catch (error) {
      console.error('Failed to load Proton versions:', error);
    }
  }

  async function handleProtonChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    selectedProtonPath = target.value;
    try {
      await invoke('set_setting', {
        key: 'proton_path',
        value: selectedProtonPath
      });
    } catch (error) {
      console.error('Failed to save Proton setting:', error);
    }
  }

  async function fetchGeReleases() {
    isFetchingReleases = true;
    try {
      geReleases = await getProtonGeReleases();
    } catch (error) {
      console.error('Failed to fetch GE-Proton releases:', error);
      alert('Failed to fetch releases: ' + error);
    } finally {
      isFetchingReleases = false;
    }
  }

  async function handleDownloadGe(release: ProtonGeRelease) {
    if (downloadingTag) return;
    downloadingTag = release.tag_name;
    downloadProgress = { stage: 'downloading', percent: 0 };
    try {
      await downloadProtonGe(release.download_url, release.tag_name);
      alert(`${release.tag_name} installed successfully!`);
      await loadProtonVersions();
      await fetchGeReleases();
    } catch (error) {
      console.error('Download failed:', error);
      alert('Download failed: ' + error);
    } finally {
      downloadingTag = null;
      downloadProgress = null;
    }
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
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
            Auto-detect your local Steam profile or log in via browser.
          </p>
        </div>
        <div style="display: flex; gap: 8px;">
          <button onclick={handleAutoDetect} class="steam-login-button" style="background: #1b2838;">
            🔍 Auto-Detect
          </button>
          <button onclick={handleSteamLogin} class="steam-login-button">
            🎮 Login with Steam
          </button>
        </div>
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

  {#if isLinux}
  <section class="settings-section">
    <h2 class="section-title text-mono text-base text-primary">Proton / Compatibility</h2>
    
    <div class="setting-item">
      <div class="setting-info">
        <p class="setting-label text-sans text-md text-primary">Default Proton Version</p>
        <p class="setting-description text-sans text-sm text-secondary">
          Select which Proton version to use for running Windows games (.exe) on Linux.
          Leave on "Auto" to use the best available version.
        </p>
      </div>
      <select class="proton-select" value={selectedProtonPath} onchange={handleProtonChange}>
        <option value="">Auto (best available)</option>
        {#each protonVersions as version}
          <option value={version.path}>
            {version.name}{version.version ? ` (${version.version})` : ''} — {version.source === 'custom' ? '🔧 Custom' : '🎮 Steam'}
          </option>
        {/each}
      </select>
    </div>

    {#if protonVersions.length === 0}
      <div class="proton-warning">
        <p class="text-sans text-sm text-secondary">
          ⚠️ No Proton versions detected. Install Proton via Steam or download 
          <a href="https://github.com/GloriousEggroll/proton-ge-custom/releases" target="_blank">GE-Proton</a> 
          to <code>~/.steam/steam/compatibilitytools.d/</code>
        </p>
      </div>
    {:else}
      <div class="proton-list">
        <p class="text-mono text-xs text-tertiary" style="margin-bottom: 8px;">DETECTED VERSIONS</p>
        {#each protonVersions as version}
          <div class="proton-item">
            <span class="proton-source-badge" class:custom={version.source === 'custom'} class:steam={version.source === 'steam'}>
              {version.source === 'custom' ? '🔧' : '🎮'}
            </span>
            <div class="proton-item-info">
              <span class="text-sans text-sm text-primary">{version.name}</span>
              {#if version.version}
                <span class="text-sans text-xs text-tertiary">{version.version}</span>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- GE-Proton Download Manager -->
    <div class="ge-download-section">
      <div class="setting-item">
        <div class="setting-info">
          <p class="setting-label text-sans text-md text-primary">Download GE-Proton</p>
          <p class="setting-description text-sans text-sm text-secondary">
            Download and install GE-Proton versions directly from GitHub.
            They will be installed to <code>~/.steam/steam/compatibilitytools.d/</code>
          </p>
        </div>
        <button onclick={fetchGeReleases} disabled={isFetchingReleases || !!downloadingTag}>
          {#if isFetchingReleases}
            Fetching...
          {:else}
            🔄 Fetch Releases
          {/if}
        </button>
      </div>

      {#if downloadProgress}
        <div class="download-progress">
          <div class="progress-info">
            <span class="text-mono text-sm text-primary">
              {downloadProgress.stage === 'downloading' ? '⬇ Downloading' : '📦 Extracting'} {downloadingTag}
            </span>
            <span class="text-mono text-xs text-tertiary">{downloadProgress.percent}%</span>
          </div>
          <div class="progress-bar-track">
            <div class="progress-bar-fill" style="width: {downloadProgress.percent}%"></div>
          </div>
        </div>
      {/if}

      {#if geReleases.length > 0}
        <div class="ge-releases-list">
          <p class="text-mono text-xs text-tertiary" style="margin-bottom: 8px;">AVAILABLE RELEASES</p>
          {#each geReleases as release}
            <div class="ge-release-item">
              <div class="ge-release-info">
                <span class="text-sans text-sm text-primary">{release.name || release.tag_name}</span>
                <span class="text-sans text-xs text-tertiary">
                  {formatSize(release.size_bytes)} · {new Date(release.published_at).toLocaleDateString()}
                </span>
              </div>
              {#if release.is_installed}
                <span class="ge-installed-badge text-mono text-xs">✅ Installed</span>
              {:else}
                <button 
                  class="ge-download-btn"
                  onclick={() => handleDownloadGe(release)}
                  disabled={!!downloadingTag}
                >
                  {downloadingTag === release.tag_name ? 'Installing...' : '⬇ Install'}
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </section>
  {/if}

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

  /* Proton Section */
  .proton-select {
    width: 300px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-border-1);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-md);
    font-family: var(--font-mono);
    color: var(--color-text-primary);
    cursor: pointer;
  }

  .proton-select:focus {
    outline: none;
    border-color: var(--color-accent);
  }

  .proton-warning {
    padding: var(--space-3);
    background: var(--color-bg-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border-1);
  }

  .proton-warning a {
    color: var(--color-accent);
  }

  .proton-warning code {
    background: var(--color-bg-3);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }

  .proton-list {
    padding: var(--space-3);
    background: var(--color-bg-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border-1);
  }

  .proton-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) 0;
  }

  .proton-item + .proton-item {
    border-top: 1px solid var(--color-border-1);
  }

  .proton-source-badge {
    font-size: 16px;
    width: 24px;
    text-align: center;
  }

  .proton-item-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  /* GE-Proton Download Section */
  .ge-download-section {
    margin-top: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .ge-download-section code {
    background: var(--color-bg-3);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-size: var(--text-xs);
  }

  .download-progress {
    padding: var(--space-3);
    background: var(--color-bg-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border-1);
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }

  .progress-bar-track {
    width: 100%;
    height: 6px;
    background: var(--color-bg-3);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: linear-gradient(90deg, #6366f1, #8b5cf6, #a78bfa);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .ge-releases-list {
    padding: var(--space-3);
    background: var(--color-bg-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border-1);
  }

  .ge-release-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) 0;
  }

  .ge-release-item + .ge-release-item {
    border-top: 1px solid var(--color-border-1);
  }

  .ge-release-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ge-installed-badge {
    color: #4ade80;
    padding: var(--space-1) var(--space-2);
  }

  .ge-download-btn {
    background: rgba(99, 102, 241, 0.2) !important;
    border: 1px solid rgba(99, 102, 241, 0.4) !important;
    color: #a5b4fc !important;
    padding: var(--space-1) var(--space-3) !important;
    font-size: var(--text-sm) !important;
    border-radius: var(--radius-md) !important;
  }

  .ge-download-btn:hover:not(:disabled) {
    background: rgba(99, 102, 241, 0.35) !important;
    border-color: rgba(99, 102, 241, 0.6) !important;
  }

  .ge-download-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
