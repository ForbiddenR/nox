import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { Run, RunEvent } from '../../lib/types'

export function useRuns(threadId: string | null | undefined) {
  return useQuery({
    queryKey: ['runs', threadId],
    queryFn: async () => {
      if (!threadId) return { runs: [] }
      const result = await window.api.invoke('list_runs', { thread_id: threadId })
      return result as { runs: Run[] }
    },
    enabled: !!threadId,
  })
}

export function useRunEvents(runId: string | null | undefined) {
  return useQuery({
    queryKey: ['run-events', runId],
    queryFn: async () => {
      if (!runId) return { events: [] }
      const result = await window.api.invoke('list_run_events', { run_id: runId })
      return result as { events: RunEvent[] }
    },
    enabled: !!runId,
    refetchInterval: (query) => {
      const data = query.state.data as { events: RunEvent[] } | undefined
      const events = data?.events || []
      // Keep polling if the last event isn't a completion event
      const lastEvent = events[events.length - 1]
      if (!lastEvent) return 1000
      if (['RunCompleted', 'RunFailed', 'RunCancelled'].includes(lastEvent.event_type)) {
        return false // Stop polling
      }
      return 1000 // Poll every second
    },
  })
}

export function useStartRun() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ threadId, prompt }: { threadId: string; prompt: string }) => {
      const result = await window.api.invoke('start_run', {
        thread_id: threadId,
        prompt,
      })
      const response = result as { ok: boolean; result?: { run_id: string }; error?: string }

      if (!response.ok || !response.result) {
        throw new Error(response.error || 'Failed to start run')
      }

      return response.result.run_id
    },
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['runs', variables.threadId] })
    },
  })
}
