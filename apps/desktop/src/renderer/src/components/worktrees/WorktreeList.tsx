import { useState } from 'react'
import type { Worktree } from '../../lib/types'
import {
  useWorktrees,
  useCreateWorktree,
  useSwitchWorktree,
  useRemoveWorktree,
} from '../../features/worktrees/useWorktrees'

interface WorktreeListProps {
  projectId: string | null
}

export default function WorktreeList({ projectId }: WorktreeListProps) {
  const [showCreateForm, setShowCreateForm] = useState(false)
  const [newWorktreeName, setNewWorktreeName] = useState('')
  const [baseRef, setBaseRef] = useState('HEAD')

  const { data: worktreesResult, isLoading } = useWorktrees(projectId)
  const createWorktree = useCreateWorktree()
  const switchWorktree = useSwitchWorktree()
  const removeWorktree = useRemoveWorktree()

  const worktrees = worktreesResult?.ok ? worktreesResult.result?.worktrees || [] : []

  const handleCreate = async () => {
    if (!projectId || !newWorktreeName.trim()) return

    try {
      await createWorktree.mutateAsync({
        projectId,
        name: newWorktreeName.trim(),
        baseRef,
      })
      setNewWorktreeName('')
      setBaseRef('HEAD')
      setShowCreateForm(false)
    } catch (error) {
      console.error('Failed to create worktree:', error)
    }
  }

  const handleSwitch = async (worktreeId: string) => {
    if (!projectId) return
    try {
      await switchWorktree.mutateAsync({ projectId, worktreeId })
    } catch (error) {
      console.error('Failed to switch worktree:', error)
    }
  }

  const handleRemove = async (worktreeId: string) => {
    if (!projectId) return
    if (!confirm('Remove this worktree? Any uncommitted changes will be lost.')) return

    try {
      await removeWorktree.mutateAsync({ projectId, worktreeId, force: true })
    } catch (error) {
      console.error('Failed to remove worktree:', error)
    }
  }

  if (!projectId) {
    return (
      <div className="p-4 text-sm text-neutral-500">
        Open a project to manage worktrees
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-3 border-b border-neutral-800">
        <h3 className="text-sm font-semibold text-neutral-200">Worktrees</h3>
        <button
          onClick={() => setShowCreateForm(!showCreateForm)}
          className="text-xs text-blue-400 hover:text-blue-300"
        >
          {showCreateForm ? 'Cancel' : '+ New'}
        </button>
      </div>

      {showCreateForm && (
        <div className="p-3 border-b border-neutral-800 bg-neutral-900/50 space-y-2">
          <input
            type="text"
            placeholder="Worktree name"
            value={newWorktreeName}
            onChange={(e) => setNewWorktreeName(e.target.value)}
            className="w-full rounded bg-neutral-800 border border-neutral-700 px-2 py-1 text-sm text-neutral-200 placeholder:text-neutral-500 focus:border-blue-500 focus:outline-none"
          />
          <input
            type="text"
            placeholder="Base ref (HEAD, main, origin/main)"
            value={baseRef}
            onChange={(e) => setBaseRef(e.target.value)}
            className="w-full rounded bg-neutral-800 border border-neutral-700 px-2 py-1 text-sm text-neutral-200 placeholder:text-neutral-500 focus:border-blue-500 focus:outline-none"
          />
          <button
            onClick={handleCreate}
            disabled={!newWorktreeName.trim() || createWorktree.isPending}
            className="w-full rounded bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {createWorktree.isPending ? 'Creating...' : 'Create Worktree'}
          </button>
        </div>
      )}

      <div className="flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="p-4 text-sm text-neutral-500">Loading...</div>
        ) : worktrees.length === 0 ? (
          <div className="p-4 text-sm text-neutral-500">
            No worktrees yet. Create one to work in isolation.
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {worktrees.map((worktree) => (
              <WorktreeItem
                key={worktree.id}
                worktree={worktree}
                onSwitch={() => handleSwitch(worktree.id)}
                onRemove={() => handleRemove(worktree.id)}
                isSwitching={switchWorktree.isPending}
                isRemoving={removeWorktree.isPending}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

interface WorktreeItemProps {
  worktree: Worktree
  onSwitch: () => void
  onRemove: () => void
  isSwitching: boolean
  isRemoving: boolean
}

function WorktreeItem({
  worktree,
  onSwitch,
  onRemove,
  isSwitching,
  isRemoving,
}: WorktreeItemProps) {
  const [showActions, setShowActions] = useState(false)

  return (
    <div
      className={`group relative rounded px-3 py-2 text-sm hover:bg-neutral-800 ${
        worktree.is_active ? 'bg-neutral-800 border-l-2 border-blue-500' : ''
      }`}
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
    >
      <div className="flex items-center justify-between">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-neutral-200 truncate">{worktree.name}</span>
            {worktree.is_active && (
              <span className="text-xs text-blue-400 font-medium">ACTIVE</span>
            )}
          </div>
          <div className="text-xs text-neutral-500 truncate">{worktree.branch}</div>
          <div className="text-xs text-neutral-600 font-mono truncate">
            {worktree.head_sha.slice(0, 7)}
          </div>
        </div>
        {showActions && !worktree.is_active && (
          <div className="flex items-center gap-1">
            <button
              onClick={onSwitch}
              disabled={isSwitching}
              className="rounded px-2 py-1 text-xs text-blue-400 hover:bg-blue-900/30 disabled:opacity-50"
              title="Switch to this worktree"
            >
              Switch
            </button>
            <button
              onClick={onRemove}
              disabled={isRemoving}
              className="rounded px-2 py-1 text-xs text-red-400 hover:bg-red-900/30 disabled:opacity-50"
              title="Remove worktree"
            >
              Remove
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
