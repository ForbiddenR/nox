import { useMutation, useQueryClient } from '@tanstack/react-query'
import type { Project } from '../../lib/types'

export function useOpenProject() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (path: string) => {
      const response = await window.api.invoke('open_project', { path })
      const result = response as { ok: boolean; result?: Project; error?: string }

      if (!result.ok || !result.result) {
        throw new Error(result.error || 'Failed to open project')
      }

      return result.result
    },
    onSuccess: () => {
      // Invalidate the recent projects query to refresh the sidebar
      queryClient.invalidateQueries({ queryKey: ['recent-projects'] })
    },
  })
}
