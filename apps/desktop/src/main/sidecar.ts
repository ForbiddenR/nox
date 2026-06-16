import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { EventEmitter } from 'node:events'

/** Protocol version the main process speaks; must match the sidecar. */
export const PROTOCOL_VERSION = 1

type Pending = {
  resolve: (result: unknown) => void
  reject: (err: Error) => void
  timer: NodeJS.Timeout
}

export type SidecarStatus = 'starting' | 'ready' | 'degraded' | 'stopped'

export interface RpcError extends Error {
  code: number
  data?: unknown
}

function rpcError(message: string, code: number, data?: unknown): RpcError {
  const err = new Error(message) as RpcError
  err.code = code
  err.data = data
  return err
}

const REQUEST_TIMEOUT_MS = 30_000
const MAX_RESTARTS = 5
const BASE_BACKOFF_MS = 500
const MAX_BACKOFF_MS = 10_000
const SHUTDOWN_GRACE_MS = 2_000

/**
 * Supervises the Rust sidecar: length-framed JSON-RPC over stdio, id-correlated
 * requests, notification routing, handshake-on-spawn, and bounded
 * restart-with-backoff. Emits:
 *  - `status` (SidecarStatus)        — lifecycle transitions
 *  - `notification` ({method, params}) — id-less events from the sidecar
 *  - `log` (string)                  — sidecar stderr lines
 */
export class Sidecar extends EventEmitter {
  private readonly binaryPath: string
  private proc: ChildProcessWithoutNullStreams | null = null
  private status: SidecarStatus = 'stopped'

  private nextId = 1
  private readonly pending = new Map<number, Pending>()

  // stdout frame accumulator
  private buffer = Buffer.alloc(0)
  private expectedLength: number | null = null

  private restarts = 0
  private restartTimer: NodeJS.Timeout | null = null
  private shuttingDown = false

  constructor(binaryPath: string) {
    super()
    this.binaryPath = binaryPath
  }

  getStatus(): SidecarStatus {
    return this.status
  }

  /** Spawn the sidecar and complete the version handshake. */
  async start(): Promise<void> {
    this.shuttingDown = false
    this.spawnProcess()
    await this.handshake()
  }

  private setStatus(status: SidecarStatus): void {
    if (this.status !== status) {
      this.status = status
      this.emit('status', status)
    }
  }

  private spawnProcess(): void {
    this.setStatus('starting')
    const proc = spawn(this.binaryPath, [], { stdio: ['pipe', 'pipe', 'pipe'] })
    this.proc = proc

    proc.stdout.on('data', (chunk: Buffer) => this.onStdout(chunk))
    proc.stderr.on('data', (chunk: Buffer) => this.emit('log', chunk.toString()))
    proc.on('error', (err) => this.emit('log', `[sidecar] spawn error: ${err.message}`))
    proc.on('exit', (code, signal) => this.onExit(code, signal))
  }

  private async handshake(): Promise<void> {
    try {
      const result = (await this.request('health/handshake', {
        protocol_version: PROTOCOL_VERSION,
      })) as { protocol_version?: number; runtime_version?: string }

      if (result?.protocol_version !== PROTOCOL_VERSION) {
        throw new Error(
          `protocol mismatch: main ${PROTOCOL_VERSION}, sidecar ${result?.protocol_version}`,
        )
      }
      this.restarts = 0
      this.setStatus('ready')
      this.emit('log', `[sidecar] handshake ok (runtime ${result.runtime_version})`)
    } catch (err) {
      this.setStatus('degraded')
      throw err
    }
  }

  /** Send an id-correlated request; resolves with `result` or rejects on error/timeout. */
  request(method: string, params?: unknown): Promise<unknown> {
    const proc = this.proc
    if (!proc || proc.exitCode !== null) {
      return Promise.reject(rpcError('sidecar not running', -32001))
    }
    const id = this.nextId++
    const payload = { jsonrpc: '2.0', id, method, params: params ?? null }

    return new Promise<unknown>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(rpcError(`request timed out: ${method}`, -32002))
      }, REQUEST_TIMEOUT_MS)

      this.pending.set(id, { resolve, reject, timer })
      this.writeMessage(proc, payload)
    })
  }

  private writeMessage(proc: ChildProcessWithoutNullStreams, message: unknown): void {
    const body = Buffer.from(JSON.stringify(message), 'utf8')
    proc.stdin.write(`Content-Length: ${body.length}\r\n\r\n`)
    proc.stdin.write(body)
  }

  // --- stdout framing -------------------------------------------------------

  private onStdout(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk])
    // Process as many complete frames as are buffered.
    for (;;) {
      if (this.expectedLength === null) {
        const headerEnd = this.buffer.indexOf('\r\n\r\n')
        if (headerEnd === -1) return
        const header = this.buffer.subarray(0, headerEnd).toString('ascii')
        const match = /content-length:\s*(\d+)/i.exec(header)
        if (!match) {
          this.emit('log', `[sidecar] malformed header: ${header}`)
          this.buffer = this.buffer.subarray(headerEnd + 4)
          continue
        }
        this.expectedLength = Number(match[1])
        this.buffer = this.buffer.subarray(headerEnd + 4)
      }
      if (this.buffer.length < this.expectedLength) return
      const body = this.buffer.subarray(0, this.expectedLength)
      this.buffer = this.buffer.subarray(this.expectedLength)
      this.expectedLength = null
      this.dispatchMessage(body)
    }
  }

  private dispatchMessage(body: Buffer): void {
    let msg: {
      id?: number
      method?: string
      params?: unknown
      result?: unknown
      error?: { code: number; message: string; data?: unknown }
    }
    try {
      msg = JSON.parse(body.toString('utf8'))
    } catch (err) {
      this.emit('log', `[sidecar] failed to parse message: ${(err as Error).message}`)
      return
    }

    // Notification: no id, has a method.
    if (msg.id === undefined || msg.id === null) {
      if (msg.method) {
        this.emit('notification', { method: msg.method, params: msg.params })
      }
      return
    }

    // Response to a pending request.
    const pending = this.pending.get(msg.id)
    if (!pending) {
      this.emit('log', `[sidecar] response for unknown id ${msg.id}`)
      return
    }
    this.pending.delete(msg.id)
    clearTimeout(pending.timer)
    if (msg.error) {
      pending.reject(rpcError(msg.error.message, msg.error.code, msg.error.data))
    } else {
      pending.resolve(msg.result)
    }
  }

  // --- lifecycle ------------------------------------------------------------

  private onExit(code: number | null, signal: NodeJS.Signals | null): void {
    this.emit('log', `[sidecar] exited (code=${code}, signal=${signal})`)
    this.proc = null
    this.buffer = Buffer.alloc(0)
    this.expectedLength = null

    // Fail all in-flight requests.
    for (const [, pending] of this.pending) {
      clearTimeout(pending.timer)
      pending.reject(rpcError('sidecar exited', -32000))
    }
    this.pending.clear()

    if (this.shuttingDown) {
      this.setStatus('stopped')
      return
    }

    this.setStatus('degraded')
    this.scheduleRestart()
  }

  private scheduleRestart(): void {
    if (this.restarts >= MAX_RESTARTS) {
      this.emit('log', `[sidecar] giving up after ${this.restarts} restarts`)
      return
    }
    const delay = Math.min(BASE_BACKOFF_MS * 2 ** this.restarts, MAX_BACKOFF_MS)
    this.restarts++
    this.emit('log', `[sidecar] restarting in ${delay}ms (attempt ${this.restarts})`)
    this.restartTimer = setTimeout(() => {
      this.restartTimer = null
      this.spawnProcess()
      this.handshake().catch((err) => this.emit('log', `[sidecar] restart handshake failed: ${err.message}`))
    }, delay)
  }

  /** Graceful shutdown: request `shutdown`, then hard-kill after a grace period.
   *  Resolves once the process has fully exited. */
  async stop(): Promise<void> {
    this.shuttingDown = true
    if (this.restartTimer) {
      clearTimeout(this.restartTimer)
      this.restartTimer = null
    }
    const proc = this.proc
    if (!proc) {
      this.setStatus('stopped')
      return
    }

    const exited = new Promise<void>((resolve) => proc.once('exit', () => resolve()))
    try {
      await Promise.race([
        this.request('shutdown'),
        new Promise((resolve) => setTimeout(resolve, SHUTDOWN_GRACE_MS)),
      ])
    } catch {
      // ignore — we kill below regardless
    }
    if (proc.exitCode === null) {
      proc.kill()
    }
    await exited
  }
}
