<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { games, ui, loadGames, runStartupSync, initSyncListener, initLaunchListener } from '$lib/stores';
  import { listen } from '@tauri-apps/api/event';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Topbar from '$lib/components/Topbar.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';

  let { children } = $props();

  onMount(async () => {
    // Initialize sync listener
    await initSyncListener();
    
    // Initialize launch state listener
    await initLaunchListener();
    
    // Listen for session updates and auto-refresh
    const unlistenSession = await listen('session_updated', (event: any) => {
      console.log('Session updated:', event.payload);
      // Auto-refresh games list to show updated playtime
      loadGames();
    });

    // Run startup sync
    await runStartupSync();
    
    // Load games
    ui.setLoading(true);
    try {
      await loadGames();
    } catch (error) {
      console.error('Failed to load games:', error);
    }
    ui.setLoading(false);

    // Sync active route
    $effect(() => {
      ui.setActiveRoute($page.route.id || '/');
    });

    // Cleanup listeners
    return () => {
      unlistenSession();
    };
  });
</script>

<div class="app-layout">
  <Sidebar />
  <main class="main-content">
    <Topbar />
    <div class="content">
      {@render children()}
    </div>
  </main>
  <StatusBar />
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
  }

  .app-layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    grid-template-rows: 1fr auto;
    height: 100vh;
    background: var(--bg-root);
  }

  .main-content {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-5);
  }
</style>
