import { app, BrowserWindow, ipcMain } from 'electron'
import { join } from 'node:path'
import { existsSync } from 'node:fs'
import { spawn, type ChildProcess } from 'node:child_process'

let sidecar: ChildProcess | null = null

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

function startSidecar(): void {
  const path = resolveSidecarPath()
  if (!existsSync(path)) {
    console.warn(`[sidecar] binary not found at ${path}; skipping spawn`)
    return
  }
  sidecar = spawn(path, [], { stdio: ['pipe', 'pipe', 'pipe'] })
  sidecar.stdout?.on('data', (d) => console.log(`[sidecar] ${d}`))
  sidecar.stderr?.on('data', (d) => console.error(`[sidecar] ${d}`))
  sidecar.on('exit', (code) => console.warn(`[sidecar] exited with code ${code}`))
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

app.on('before-quit', () => {
  sidecar?.kill()
})

// Placeholder RPC router — forwards renderer calls to the sidecar (wired up in M1).
ipcMain.handle('rpc', async (_event, method: string, _params?: unknown) => {
  return { ok: false, error: `rpc not implemented: ${method}` }
})
