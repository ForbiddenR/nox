import { useState } from 'react'
import { useProjectStore } from '../../store/project'
import { useThreadStore } from '../../store/thread'
import {
  useThreads,
  useCreateThread,
  useRenameThread,
  useArchiveThread,
} from '../../features/threads/useThreads'

export default function ThreadList() {
  const { currentProject } = useProjectStore()
  const { currentThread, setCurrentThread } = useThreadStore()
  const [editingThreadId, setEditingThreadId] = useState<string | null>(null)
  const [editTitle, setEditTitle] = useState('')

  const { data: threadsData } = useThreads(currentProject?.id)
  const createThread = useCreateThread()
  const renameThread = useRenameThread()
  const archiveThread = useArchiveThread()

  const threads = threadsData?.threads || []

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

  const handleRename = async (threadId: string) => {
    if (!currentProject || !editTitle.trim()) return

    try {
      await renameThread.mutateAsync({
        threadId,
        title: editTitle.trim(),
        projectId: currentProject.id,
      })
      setEditingThreadId(null)
      setEditTitle('')
    } catch (err) {
      console.error('Failed to rename thread:', err)
    }
  }

  const handleArchive = async (threadId: string) => {
    if (!currentProject) return

    try {
      await archiveThread.mutateAsync({
        threadId,
        projectId: currentProject.id,
      })
      if (currentThread?.id === threadId) {
        setCurrentThread(null)
      }
    } catch (err) {
      console.error('Failed to archive thread:', err)
    }
  }

  const startEditing = (thread: { id: string; title: string }) => {
    setEditingThreadId(thread.id)
    setEditTitle(thread.title)
  }

  if (!currentProject) {
    return null
  }

  return (
    <div className="flex flex-col border-b border-neutral-800">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3">
        <h3 className="text-sm font-semibold text-neutral-300">Threads</h3>
        <button
          onClick={handleCreateThread}
          disabled={createThread.isPending}
          className="text-blue-500 hover:text-blue-400 disabled:opacity-50"
          title="Create thread"
        >
          <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
        </button>
      </div>

      {/* Thread list */}
      <div className="max-h-64 overflow-y-auto px-2 pb-2">
        {threads.length === 0 ? (
          <div className="px-3 py-4 text-center text-xs text-neutral-500">
            No threads yet
          </div>
        ) : (
          <div className="space-y-1">
            {threads.map((thread) => (
              <div
                key={thread.id}
                className={`group relative rounded-md px-3 py-2 text-sm ${
                  currentThread?.id === thread.id
                    ? 'bg-neutral-800 text-white'
                    : 'text-neutral-300 hover:bg-neutral-800/50'
                }`}
              >
                {editingThreadId === thread.id ? (
                  <form
                    onSubmit={(e) => {
                      e.preventDefault()
                      handleRename(thread.id)
                    }}
                    className="flex gap-1"
                  >
                    <input
                      type="text"
                      value={editTitle}
                      onChange={(e) => setEditTitle(e.target.value)}
                      onBlur={() => handleRename(thread.id)}
                      autoFocus
                      className="flex-1 bg-neutral-900 px-2 py-1 text-sm text-white outline-none"
                    />
                  </form>
                ) : (
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setCurrentThread(thread)}
                      className="flex-1 truncate text-left"
                    >
                      {thread.title}
                    </button>
                    <div className="flex gap-1 opacity-0 group-hover:opacity-100">
                      <button
                        onClick={() => startEditing(thread)}
                        className="text-neutral-400 hover:text-neutral-200"
                        title="Rename"
                      >
                        <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                        </svg>
                      </button>
                      <button
                        onClick={() => handleArchive(thread.id)}
                        className="text-neutral-400 hover:text-red-400"
                        title="Archive"
                      >
                        <svg className="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
                        </svg>
                      </button>
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
