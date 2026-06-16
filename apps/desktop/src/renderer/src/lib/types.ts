// Type definitions for the RPC API exposed by the preload script

export interface Project {
  id: string
  name: string
  path: string
  is_git_repo: boolean
  current_branch: string | null
  created_at: string
  last_opened_at: string
}

export interface RepoStatusSummary {
  branch: string
  dirty: boolean
  ahead: number
  behind: number
}

export interface Thread {
  id: string
  project_id: string
  title: string
  archived: boolean
  created_at: string
  updated_at: string
}

export interface RpcError {
  code: number
  message: string
  data?: unknown
}

declare global {
  interface Window {
    api: {
      invoke: (method: string, params?: unknown) => Promise<unknown>
      subscribe: (
        method: string,
        callback: (params: unknown) => void,
      ) => () => void
      dialog: {
        openFolder: () => Promise<string | null>
      }
    }
  }
}
