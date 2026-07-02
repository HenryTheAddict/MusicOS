const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("musicos", {
  getConfig: () => ipcRenderer.invoke("config:get"),
  setTarget: target => ipcRenderer.invoke("config:set-target", target),
  chooseFolder: () => ipcRenderer.invoke("folder:choose"),
  openFolder: () => ipcRenderer.invoke("folder:open"),
  openCoverArtCache: () => ipcRenderer.invoke("folder:openCoverArtCache"),
  uploadFolder: () => ipcRenderer.invoke("folder:upload"),
  resetDb: () => ipcRenderer.invoke("library:resetDb"),
  backupLibrary: () => ipcRenderer.invoke("library:backup"),
  restoreLibrary: () => ipcRenderer.invoke("library:restore"),
  exportPlaylists: () => ipcRenderer.invoke("library:playlistExport"),
  importPlaylists: () => ipcRenderer.invoke("library:playlistImport"),
  exportDiagnostics: () => ipcRenderer.invoke("diagnostics:export"),
  performance: {
    smoke: () => ipcRenderer.invoke("performance:smoke"),
    report: () => ipcRenderer.invoke("performance:report:get")
  },
  sync: {
    start: files => ipcRenderer.invoke("sync:start", files),
    status: () => ipcRenderer.invoke("sync:status"),
    dryRun: files => ipcRenderer.invoke("sync:dryRun", files),
    retryFailed: () => ipcRenderer.invoke("sync:retryFailed"),
    cancel: () => ipcRenderer.invoke("sync:cancel"),
    history: () => ipcRenderer.invoke("sync:history:get"),
    clearHistory: () => ipcRenderer.invoke("sync:history:clear")
  },
  emulator: {
    start: () => ipcRenderer.invoke("emulator:start"),
    stop: () => ipcRenderer.invoke("emulator:stop"),
    restart: () => ipcRenderer.invoke("emulator:restart"),
    status: () => ipcRenderer.invoke("emulator:status"),
    logs: () => ipcRenderer.invoke("emulator:logs:get"),
    clearLogs: () => ipcRenderer.invoke("emulator:logs:clear")
  }
});
