const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("musicos", {
  getConfig: () => ipcRenderer.invoke("config:get"),
  setTarget: (target) => ipcRenderer.invoke("config:set-target", target),
  chooseFolder: () => ipcRenderer.invoke("folder:choose"),
  uploadFolder: (customPath) => ipcRenderer.invoke("folder:upload", customPath),
  onUploadProgress: (callback) => {
    const handler = (_event, data) => callback(data);
    ipcRenderer.on("upload:progress", handler);
    return () => ipcRenderer.removeListener("upload:progress", handler);
  },
  pauseUpload: () => ipcRenderer.send("upload:pause"),
  resumeUpload: () => ipcRenderer.send("upload:resume"),
});
