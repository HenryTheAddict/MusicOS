const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("musicos", {
  getConfig: () => ipcRenderer.invoke("config:get"),
  setTarget: (target) => ipcRenderer.invoke("config:set-target", target),
  chooseFolder: () => ipcRenderer.invoke("folder:choose"),
  uploadFolder: () => ipcRenderer.invoke("folder:upload")
});
