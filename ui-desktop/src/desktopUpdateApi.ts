import {Channel, invoke, isTauri} from '@tauri-apps/api/core';

export type UpdateStatus = {version: string; available: boolean; ready: string | null; error: string | null};
export type DownloadProgress =
  | {phase: 'downloading'; received: number; total: number | null}
  | {phase: 'verifying'};

export const desktopUpdateApi = {
  status: (): Promise<UpdateStatus | null> =>
    isTauri() ? invoke('desktop_update_status') : Promise.resolve(null),
  check: (): Promise<string | null> => invoke('desktop_update_check'),
  download: (version: string, onProgress: (progress: DownloadProgress) => void): Promise<void> =>
    invoke('desktop_update_download', {version, progress: new Channel(onProgress)}),
  restart: (version: string): Promise<void> => invoke('desktop_update_restart', {version}),
};
