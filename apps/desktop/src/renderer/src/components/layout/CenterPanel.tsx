import { useProjectStore } from '../../store/project'
import { useThreadStore } from '../../store/thread'
import { useCreateThread } from '../../features/threads/useThreads'
import { useRuns, useRunEvents, useStartRun } from '../../features/runs/useRuns'
import Timeline from '../timeline/Timeline'
import PromptInput from '../timeline/PromptInput'

export default function CenterPanel() {
  const { currentProject } = useProjectStore()
  const { currentThread, setCurrentThread } = useThreadStore()
  const createThread = useCreateThread()
  const startRun = useStartRun()

  // Load runs for the current thread
  const { data: runsData } = useRuns(currentThread?.id)
  const runs = runsData?.runs || []
  const latestRun = runs[0] // Runs are sorted by created_at DESC

  // Load events for the latest run
  const { data: eventsData } = useRunEvents(latestRun?.id)
  const events = eventsData?.events || []

  const handleCreateThread = async () => {
    if (!currentProject) return

    try {
      const thread = await createThread.mutateAsync({
        projectId: currentProject.id,
      })
      setCurrentThread(thread)
    } catch (err) {
      console.error('Failed to create thread:', err)
    }
  }

  const handleSendPrompt = async (prompt: string) => {
    if (!currentThread) return

    try {
      await startRun.mutateAsync({
        threadId: currentThread.id,
        prompt,
      })
    } catch (err) {
      console.error('Failed to start run:', err)
    }
  }

  const isRunning = latestRun?.state === 'running' || latestRun?.state === 'queued'

  if (!currentProject) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <h2 className="text-xl font-semibold text-neutral-300">
            No project selected
          </h2>
          <p className="mt-2 text-sm text-neutral-500">
            Open a project from the sidebar to get started
          </p>
        </div>
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col">
      {/* Project header */}
      <div className="border-b border-neutral-800 px-6 py-4">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-600/10">
            <svg className="h-5 w-5 text-blue-500" fill="currentColor" viewBox="0 0 24 24">
              <path d="M20 6h-8l-2-2H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2z" />
            </svg>
          </div>
          <div className="flex-1">
            <h1 className="text-lg font-semibold">{currentProject.name}</h1>
            <div className="flex items-center gap-4 text-sm text-neutral-400">
              <span className="truncate">{currentProject.path}</span>
              {currentProject.current_branch && (
                <>
                  <span>•</span>
                  <span className="flex items-center gap-1">
                    <svg className="h-3.5 w-3.5" fill="currentColor" viewBox="0 0 16 16">
                      <path fillRule="evenodd" d="M11.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122V6A2.5 2.5 0 0110 8.5H6a1 1 0 00-1 1v1.128a2.251 2.251 0 11-1.5 0V5.372a2.25 2.25 0 111.5 0v1.836A2.492 2.492 0 016 7h4a1 1 0 001-1v-.628A2.25 2.25 0 019.5 3.25zM4.25 12a.75.75 0 100 1.5.75.75 0 000-1.5zM3.5 3.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0z" />
                    </svg>
                    {currentProject.current_branch}
                  </span>
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Transcript/timeline area */}
      <div className="flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-4xl">
          {currentThread ? (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-semibold">{currentThread.title}</h2>
                <span className="text-xs text-neutral-500">
                  {new Date(currentThread.created_at).toLocaleDateString()}
                </span>
              </div>

              {/* Timeline */}
              {latestRun && events.length > 0 ? (
                <div className="space-y-6">
                  <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6">
                    <div className="mb-4 flex items-center justify-between">
                      <span className="text-sm text-neutral-400">Prompt:</span>
                      <span
                        className={`rounded-full px-2 py-1 text-xs ${
                          latestRun.state === 'completed'
                            ? 'bg-green-900/50 text-green-400'
                            : latestRun.state === 'running'
                              ? 'bg-blue-900/50 text-blue-400'
                              : latestRun.state === 'failed'
                                ? 'bg-red-900/50 text-red-400'
                                : 'bg-neutral-800 text-neutral-400'
                        }`}
                      >
                        {latestRun.state}
                      </span>
                    </div>
                    <p className="mb-6 text-neutral-300">{latestRun.prompt}</p>
                    <Timeline runId={latestRun.id} events={events} />
                  </div>

                  {/* Show older runs if any */}
                  {runs.length > 1 && (
                    <div className="text-center text-sm text-neutral-500">
                      {runs.length - 1} older run{runs.length > 2 ? 's' : ''} in this thread
                    </div>
                  )}
                </div>
              ) : (
                <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6 text-center">
                  <p className="text-sm text-neutral-500">
                    No messages yet. Send a prompt to start.
                  </p>
                </div>
              )}
            </div>
          ) : (
            <div className="rounded-lg border border-neutral-800 bg-neutral-900/50 p-6 text-center">
              <h3 className="text-lg font-medium text-neutral-300">
                No thread selected
              </h3>
              <p className="mt-2 text-sm text-neutral-500">
                Create or select a thread to start a conversation with the agent
              </p>
              <button
                onClick={handleCreateThread}
                disabled={createThread.isPending}
                className="mt-4 rounded-md bg-neutral-800 px-4 py-2 text-sm font-medium text-neutral-200 hover:bg-neutral-700 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {createThread.isPending ? 'Creating...' : 'Create Thread'}
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Prompt input (only show when thread is selected) */}
      {currentThread && (
        <div className="border-t border-neutral-800 bg-neutral-900/50 p-4">
          <div className="mx-auto max-w-4xl">
            <PromptInput onSubmit={handleSendPrompt} disabled={isRunning} />
            {isRunning && (
              <p className="mt-2 text-xs text-neutral-500">
                Agent is working... Please wait for the current run to complete.
              </p>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
