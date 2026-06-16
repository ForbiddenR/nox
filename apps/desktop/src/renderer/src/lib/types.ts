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

export interface Run {
  id: string
  thread_id: string
  prompt: string
  state: 'queued' | 'running' | 'waiting_approval' | 'completed' | 'failed' | 'cancelled'
  worktree_id: string | null
  created_at: string
  completed_at: string | null
}

export interface RunEvent {
  id: string
  run_id: string
  sequence: number
  event_type: string
  payload: unknown
  created_at: string
}

export interface DiffSummary {
  files_changed: number
  insertions: number
  deletions: number
}

export interface ChangedFile {
  path: string
  status: 'added' | 'modified' | 'deleted' | 'renamed'
  insertions: number
  deletions: number
}

export interface Artifact {
  id: string
  run_id: string
  artifact_type: 'diff' | 'patch' | 'log' | 'summary'
  file_path: string
  created_at: string
}

export interface Worktree {
  id: string
  project_id: string
  name: string
  path: string
  branch: string
  head_sha: string
  is_active: boolean
  created_at: string
}
