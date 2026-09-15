import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type FtpSettings = {
  host: string;
  port: number;
  username: string;
  password: string;
  remotePath: string;
  passive: boolean;
};

export type Settings = {
  arma3Path: string;
  modsPath: string;
  ftp: FtpSettings;
  selectedMods: string[];
};

export type RemoteMod = {
  name: string;
  remoteBytes: number;
};

export type SyncStatus = {
  ready: boolean;
  bytesMissing: number;
  filesMissing: number;
};

export type DownloadProgress = {
  phase: "listing" | "downloading" | "done" | "error";
  modName?: string | null;
  currentFile?: string | null;
  bytesDone: number;
  bytesTotal: number;
  filesDone: number;
  filesTotal: number;
  message?: string | null;
};

export const defaultSettings = (): Settings => ({
  arma3Path: "",
  modsPath: "",
  ftp: {
    host: "",
    port: 21,
    username: "",
    password: "",
    remotePath: "/",
    passive: true,
  },
  selectedMods: [],
});

export const launcherApi = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  detectArmaPath: () => invoke<string | null>("detect_arma_path"),
  validateArmaPath: (path: string) => invoke<string>("validate_arma_path", { path }),
  launchGame: () => invoke<void>("launch_game"),
  ftpTestConnection: () => invoke<void>("ftp_test_connection"),
  ftpListMods: () => invoke<RemoteMod[]>("ftp_list_mods"),
  syncStatus: () => invoke<SyncStatus>("sync_status"),
  startSync: () => invoke<void>("start_sync"),
  cancelSync: () => invoke<void>("cancel_sync"),
  fetchWeekends: () => invoke<unknown>("fetch_weekends"),
  onDownloadProgress: (handler: (payload: DownloadProgress) => void) =>
    listen<DownloadProgress>("download-progress", (event) => handler(event.payload)) as Promise<UnlistenFn>,
};
