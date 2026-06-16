import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { Thread } from '../../lib/types'

export function useThreads(projectId: string | null | undefined) {
  return useQuery({
    queryKey: ['threads', projectId],
    queryFn: async () => {
      if (!projectId) return { threads: [] }
      const result = await window.api.invoke('list_threads', {
        project_id: projectId,
        include_archived: false,
      })
      return result as { threads: Thread[] }
    },
    enabled: !!projectId,
  })
}

export function useCreateThread() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      projectId,
      title,
    }: {
      projectId: string
      title?: string
    }) => {
      const result = await window.api.invoke('create_thread', {
        project_id: projectId,
        title,
      })
      const response = result as { ok: boolean; result?: Thread; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to create thread')
      }

      return response.result
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['threads', variables.projectId] })
    },
  })
}

export function useRenameThread() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      threadId,
      title,
      projectId,
    }: {
      threadId: string
      title: string
      projectId: string
    }) => {
      const result = await window.api.invoke('rename_thread', {
        thread_id: threadId,
        title,
      })
      const response = result as { ok: boolean; result?: Thread; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to rename thread')
      }

      return { thread: response.result, projectId }
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['threads', data.projectId] })
    },
  })
}

export function useArchiveThread() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      threadId,
      projectId,
    }: {
      threadId: string
      projectId: string
    }) => {
      const result = await window.api.invoke('archive_thread', {
        thread_id: threadId,
      })
      const response = result as { ok: boolean; result?: Thread; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to archive thread')
      }

      return { thread: response.result, projectId }
    },
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['threads', data.projectId] })
    },
  })
}
