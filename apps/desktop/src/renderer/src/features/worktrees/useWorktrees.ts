import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { Worktree } from '../../lib/types'

interface WorktreesResponse {
  worktrees: Worktree[]
}

export function useWorktrees(projectId: string | null | undefined) {
  return useQuery({
    queryKey: ['worktrees', projectId],
    queryFn: async () => {
      if (!projectId) return null
      const result = await window.api.invoke('list_worktrees', { project_id: projectId })
      return result as { ok: boolean; result?: WorktreesResponse; error?: string }
    },
    enabled: !!projectId,
  })
}

export function useCreateWorktree() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      projectId,
      name,
      baseRef,
    }: {
      projectId: string
      name: string
      baseRef?: string
    }) => {
      const result = await window.api.invoke('create_worktree', {
        project_id: projectId,
        name,
        base_ref: baseRef || 'HEAD',
      })
      const response = result as { ok: boolean; result?: { worktree: Worktree }; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to create worktree')
      }

      return response.result.worktree
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['worktrees', variables.projectId] })
    },
  })
}

export function useSwitchWorktree() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      projectId,
      worktreeId,
    }: {
      projectId: string
      worktreeId: string
    }) => {
      const result = await window.api.invoke('switch_worktree', {
        worktree_id: worktreeId,
      })
      const response = result as { ok: boolean; result?: { ok: boolean }; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to switch worktree')
      }

      return response.result
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['worktrees', variables.projectId] })
    },
  })
}

export function useRemoveWorktree() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      projectId,
      worktreeId,
      force,
    }: {
      projectId: string
      worktreeId: string
      force?: boolean
    }) => {
      const result = await window.api.invoke('remove_worktree', {
        worktree_id: worktreeId,
        force: force || false,
      })
      const response = result as { ok: boolean; result?: { ok: boolean }; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to remove worktree')
      }

      return response.result
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['worktrees', variables.projectId] })
    },
  })
}
