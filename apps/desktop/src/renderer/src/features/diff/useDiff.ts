import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { ChangedFile, DiffSummary } from '../../lib/types'

interface DiffResponse {
  diff: string
  summary: DiffSummary
  files: ChangedFile[]
}

export function useDiff(projectId: string | null | undefined) {
  return useQuery({
    queryKey: ['diff', projectId],
    queryFn: async () => {
      if (!projectId) return null
      const result = await window.api.invoke('get_diff', { project_id: projectId })
      return result as { ok: boolean; result?: DiffResponse; error?: string }
    },
    enabled: !!projectId,
    refetchInterval: 5000, // Poll every 5 seconds for changes
  })
}

export function useChangedFiles(projectId: string | null | undefined) {
  return useQuery({
    queryKey: ['changed-files', projectId],
    queryFn: async () => {
      if (!projectId) return { files: [] }
      const result = await window.api.invoke('list_changed_files', { project_id: projectId })
      return result as { ok: boolean; result?: { files: ChangedFile[] }; error?: string }
    },
    enabled: !!projectId,
  })
}

export function useApplyPatch() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ projectId, patch }: { projectId: string; patch: string }) => {
      const result = await window.api.invoke('apply_patch', {
        project_id: projectId,
        patch,
      })
      const response = result as { ok: boolean; result?: { ok: boolean }; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to apply patch')
      }

      return response.result
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['diff', variables.projectId] })
      queryClient.invalidateQueries({ queryKey: ['changed-files', variables.projectId] })
    },
  })
}

export function useRejectChanges() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ projectId }: { projectId: string }) => {
      const result = await window.api.invoke('reject_changes', {
        project_id: projectId,
      })
      const response = result as { ok: boolean; result?: { ok: boolean }; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to reject changes')
      }

      return response.result
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['diff', variables.projectId] })
      queryClient.invalidateQueries({ queryKey: ['changed-files', variables.projectId] })
    },
  })
}
