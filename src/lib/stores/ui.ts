import { writable } from 'svelte/store';

interface UIState {
  sidebarOpen: boolean;
  searchQuery: string;
  activeRoute: string;
  isLoading: boolean;
  contextMenuGameId: string | null;
}

function createUIStore() {
  const { subscribe, set, update } = writable<UIState>({
    sidebarOpen: true,
    searchQuery: '',
    activeRoute: '/',
    isLoading: false,
    contextMenuGameId: null,
  });

  return {
    subscribe,
    set,
    toggleSidebar: () => update((state) => ({ ...state, sidebarOpen: !state.sidebarOpen })),
    setSidebarOpen: (open: boolean) => update((state) => ({ ...state, sidebarOpen: open })),
    setSearchQuery: (query: string) => update((state) => ({ ...state, searchQuery: query })),
    setActiveRoute: (route: string) => update((state) => ({ ...state, activeRoute: route })),
    setLoading: (loading: boolean) => update((state) => ({ ...state, isLoading: loading })),
    setContextMenuGameId: (gameId: string | null) => update((state) => ({ ...state, contextMenuGameId: gameId })),
  };
}

export const ui = createUIStore();
