import { writable } from 'svelte/store';
import type { Game } from '$lib/types';

function createGamesStore() {
  const { subscribe, set, update } = writable<Game[]>([]);

  return {
    subscribe,
    set,
    addGame: (game: Game) => update((games) => [game, ...games]),
    removeGame: (gameId: string) => update((games) => games.filter((g) => g.id !== gameId)),
    updateGame: (gameId: string, updates: Partial<Game>) =>
      update((games) =>
        games.map((g) => (g.id === gameId ? { ...g, ...updates } : g))
      ),
  };
}

export const games = createGamesStore();
