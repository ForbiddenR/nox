import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron'

/** Typed, minimal surface exposed to the renderer. No Node/RPC details leak through. */
const api = {
  invoke: (method: string, params?: unknown) => ipcRenderer.invoke('rpc', method, params),
  subscribe: (channel: string, handler: (payload: unknown) => void) => {
    const listener = (_e: IpcRendererEvent, payload: unknown) => handler(payload)
    ipcRenderer.on(channel, listener)
    return () => ipcRenderer.removeListener(channel, listener)
  },
  dialog: {
    openFolder: () => ipcRenderer.invoke('dialog:openFolder') as Promise<string | null>,
  },
  getVersion: () => ipcRenderer.invoke('app:getVersion') as Promise<string>,
}

contextBridge.exposeInMainWorld('api', api)

export type Api = typeof api
