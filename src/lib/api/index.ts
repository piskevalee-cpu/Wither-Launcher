import { invoke } from '@tauri-apps/api/core';
import type { Game, AddGamePayload, Session, SyncResult, LaunchResult, SteamUser } from '$lib/types';

export async function getAllGames(): Promise<Game[]> {
  return invoke<Game[]>('get_all_games');
}

export async function addCustomGame(payload: AddGamePayload): Promise<Game> {
  return invoke<Game>('add_custom_game', {
    executablePath: payload.executable_path,
    name: payload.name ?? null,
    launchArgs: payload.launch_args ?? null
  });
}

export async function removeGame(gameId: string): Promise<void> {
  return invoke('remove_game', { gameId });
}

export async function launchGame(gameId: string): Promise<LaunchResult> {
  return invoke<LaunchResult>('launch_game', { gameId });
}

export async function killGame(gameId: string): Promise<void> {
  return invoke('kill_game', { gameId });
}

export async function steamLogin(): Promise<SteamUser> {
  return invoke<SteamUser>('steam_login');
}

export async function steamLogout(): Promise<void> {
  return invoke('steam_logout');
}

export async function getSteamUser(): Promise<SteamUser | null> {
  return invoke<SteamUser | null>('get_steam_user');
}

export async function syncSteam(): Promise<SyncResult> {
  return invoke<SyncResult>('sync_steam');
}

export async function getSteamGames(): Promise<unknown[]> {
  return invoke<unknown[]>('get_steam_games');
}

export async function getSessions(gameId?: string): Promise<Session[]> {
  return invoke<Session[]>('get_sessions', { gameId });
}

export async function getPlaytime(gameId: string): Promise<number> {
  return invoke<number>('get_playtime', { gameId });
}

export async function resetSteamGames(): Promise<number> {
  return invoke<number>('reset_steam_games');
}

export async function clearRemovedSteamGames(): Promise<number> {
  return invoke<number>('clear_removed_steam_games');
}

export async function readAcfFile(appId: number): Promise<string> {
  return invoke<string>('read_acf_file', { appId });
}

export interface ProtonVersion {
  name: string;
  path: string;
  version: string | null;
  source: string;
}

export async function getProtonVersions(): Promise<ProtonVersion[]> {
  return invoke<ProtonVersion[]>('get_proton_versions');
}

export interface ProtonGeRelease {
  tag_name: string;
  name: string;
  download_url: string;
  size_bytes: number;
  published_at: string;
  is_installed: boolean;
}

export async function getProtonGeReleases(): Promise<ProtonGeRelease[]> {
  return invoke<ProtonGeRelease[]>('get_proton_ge_releases');
}

export async function downloadProtonGe(downloadUrl: string, tagName: string): Promise<string> {
  return invoke<string>('download_proton_ge', { downloadUrl, tagName });
}
