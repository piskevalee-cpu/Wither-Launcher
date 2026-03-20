export interface Game {
  id: string;
  name: string;
  source: 'steam' | 'custom';
  drm_type: 'steam' | 'none';
  launch_method: 'steam_protocol' | 'executable';
  executable_path: string | null;
  steam_app_id: number | null;
  cover_url: string | null;
  background_url: string | null;
  genre: string | null;
  developer: string | null;
  release_year: number | null;
  is_installed: boolean;
  is_favourite: boolean;
  added_at: number;
  wither_playtime_s: number;
  steam_playtime_s: number;
  last_played_at: number;
  session_count: number;
}

export interface Session {
  id: string;
  game_id: string;
  started_at: number;
  ended_at: number | null;
  duration_s: number | null;
  was_crashed: boolean;
}

export interface AddGamePayload {
  name?: string;
  executable_path: string;
  cover_path?: string;
  launch_args?: string[];
}

export interface SyncResult {
  added: number;
  updated: number;
  removed: number;
  errors: string[];
  synced_at: number;
}

export interface GameEvent {
  game_id: string;
  session_id: string;
  timestamp: number;
}

export interface LaunchResult {
  session_id: string;
  pid: number | null;
  started_at: number;
}

export interface SteamUser {
  steamid: string;
  personaname: string;
  avatar: string;
  avatarfull: string;
  profileurl: string;
  is_public: boolean;
}
