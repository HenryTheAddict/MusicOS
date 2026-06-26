const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("musicos", {
  getConfig: () => ipcRenderer.invoke("config:get"),
  setTarget: target => ipcRenderer.invoke("config:set-target", target),
  chooseFolder: () => ipcRenderer.invoke("folder:choose"),
  openFolder: () => ipcRenderer.invoke("folder:open"),
  uploadFolder: () => ipcRenderer.invoke("folder:upload"),
  resetDb: () => ipcRenderer.invoke("library:resetDb"),
  emulator: {
    start: () => ipcRenderer.invoke("emulator:start"),
    stop: () => ipcRenderer.invoke("emulator:stop"),
    restart: () => ipcRenderer.invoke("emulator:restart"),
    status: () => ipcRenderer.invoke("emulator:status"),
    logs: () => ipcRenderer.invoke("emulator:logs:get"),
    clearLogs: () => ipcRenderer.invoke("emulator:logs:clear")
  }
});
