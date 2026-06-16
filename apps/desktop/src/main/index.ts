import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import { join } from 'node:path'
import { existsSync } from 'node:fs'
import { Sidecar } from './sidecar'

let sidecar: Sidecar | null = null

/** Resolve the bundled Rust sidecar binary (packaged) or the dev build (cargo). */
function resolveSidecarPath(): string {
  const exe = process.platform === 'win32' ? 'runtime-host.exe' : 'runtime-host'
  if (app.isPackaged) {
    return join(process.resourcesPath, exe)
  }
  // dev: apps/desktop/out/main -> repo root -> target/{release,debug}
  const release = join(__dirname, '../../../../target/release', exe)
  const debug = join(__dirname, '../../../../target/debug', exe)
  return existsSync(release) ? release : debug
}

/** Broadcast a payload to every renderer window on a subscription channel. */
function broadcast(channel: string, payload: unknown): void {
  for (const win of BrowserWindow.getAllWindows()) {
    win.webContents.send(channel, payload)
  }
}

function startSidecar(): void {
  const path = resolveSidecarPath()
  if (!existsSync(path)) {
    console.warn(`[sidecar] binary not found at ${path}; skipping spawn`)
    return
  }

  const instance = new Sidecar(path)
  sidecar = instance

  instance.on('log', (line: string) => console.error(line.trimEnd()))
  instance.on('status', (status: string) => {
    // Surface degraded/recovered transitions to the renderer.
    broadcast('runtime:status', { status })
    if (status === 'degraded') broadcast('runtime:degraded', {})
  })
  // Route id-less sidecar events (run:*, terminal:output, …) to renderers.
  instance.on('notification', ({ method, params }: { method: string; params: unknown }) => {
    broadcast(method, params)
  })

  instance.start().catch((err) => {
    console.error(`[sidecar] failed to start: ${err.message}`)
  })
}

function createWindow(): void {
  const win = new BrowserWindow({
    width: 1280,
    height: 800,
    show: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  })

  win.on('ready-to-show', () => win.show())

  if (process.env['ELECTRON_RENDERER_URL']) {
    win.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

app.whenReady().then(() => {
  startSidecar()
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('before-quit', (event) => {
  if (sidecar && sidecar.getStatus() !== 'stopped') {
    event.preventDefault()
    const instance = sidecar
    sidecar = null
    instance.stop().finally(() => app.quit())
  }
})

// RPC router: forward renderer `invoke(method, params)` to the sidecar.
ipcMain.handle('rpc', async (_event, method: string, params?: unknown) => {
  if (!sidecar) {
    return { ok: false, error: 'sidecar unavailable' }
  }
  try {
    const result = await sidecar.request(method, params)
    return { ok: true, result }
  } catch (err) {
    const e = err as { message?: string; code?: number; data?: unknown }
    return { ok: false, error: e.message ?? 'rpc error', code: e.code, data: e.data }
  }
})

// Native folder picker: returns the selected directory path or null if cancelled.
ipcMain.handle('dialog:openFolder', async (event) => {
  const win = BrowserWindow.fromWebContents(event.sender)
  if (!win) return null

  const result = await dialog.showOpenDialog(win, {
    properties: ['openDirectory', 'createDirectory'],
    title: 'Open Project Folder',
  })

  if (result.canceled || result.filePaths.length === 0) {
    return null
  }

  return result.filePaths[0]
})
