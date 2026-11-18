import {Channel, invoke, isTauri} from '@tauri-apps/api/core';

export type UpdateStatus = {version: string; available: boolean};
export type InstallProgress =
  | {phase: 'downloading'; received: number; total: number | null}
  | {phase: 'installing'};

export const desktopUpdateApi = {
  status: (): Promise<UpdateStatus | null> =>
    isTauri() ? invoke('desktop_update_status') : Promise.resolve(null),
  check: (): Promise<string | null> => invoke('desktop_update_check'),
  install: (version: string, onProgress: (progress: InstallProgress) => void): Promise<void> =>
    invoke('desktop_update_install', {version, progress: new Channel(onProgress)}),
};
