import { useState } from 'react'
import type { ChangedFile, DiffSummary } from '../../lib/types'

interface DiffViewProps {
  diff: string
  summary: DiffSummary
  files: ChangedFile[]
  onApply: () => void
  onReject: () => void
  isApplying?: boolean
  isRejecting?: boolean
}

export default function DiffView({
  diff,
  summary,
  files,
  onApply,
  onReject,
  isApplying,
  isRejecting,
}: DiffViewProps) {
  const [selectedFile, setSelectedFile] = useState<string | null>(null)

  const getStatusColor = (status: ChangedFile['status']) => {
    switch (status) {
      case 'added':
        return 'text-green-400'
      case 'deleted':
        return 'text-red-400'
      case 'modified':
        return 'text-yellow-400'
      case 'renamed':
        return 'text-blue-400'
    }
  }

  const getStatusIcon = (status: ChangedFile['status']) => {
    switch (status) {
      case 'added':
        return '+'
      case 'deleted':
        return '−'
      case 'modified':
        return '●'
      case 'renamed':
        return '→'
    }
  }

  return (
    <div className="flex h-full flex-col">
      {/* Summary Bar */}
      <div className="border-b border-neutral-800 bg-neutral-900/50 p-4">
        <div className="flex items-center justify-between">
          <div className="flex gap-6 text-sm">
            <span className="text-neutral-400">
              {summary.files_changed} file{summary.files_changed !== 1 ? 's' : ''} changed
            </span>
            <span className="text-green-400">+{summary.insertions}</span>
            <span className="text-red-400">−{summary.deletions}</span>
          </div>
          <div className="flex gap-2">
            <button
              onClick={onReject}
              disabled={isRejecting || isApplying}
              className="rounded-md border border-red-700 bg-red-900/20 px-4 py-2 text-sm font-medium text-red-400 hover:bg-red-900/40 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isRejecting ? 'Rejecting...' : 'Reject All'}
            </button>
            <button
              onClick={onApply}
              disabled={isApplying || isRejecting}
              className="rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {isApplying ? 'Applying...' : 'Apply Changes'}
            </button>
          </div>
        </div>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* File List */}
        <div className="w-64 border-r border-neutral-800 bg-neutral-900/30 overflow-y-auto">
          <div className="p-2 space-y-1">
            {files.map((file) => (
              <button
                key={file.path}
                onClick={() => setSelectedFile(file.path)}
                className={`w-full rounded px-3 py-2 text-left text-sm hover:bg-neutral-800 ${
                  selectedFile === file.path ? 'bg-neutral-800' : ''
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className={`font-mono ${getStatusColor(file.status)}`}>
                    {getStatusIcon(file.status)}
                  </span>
                  <span className="truncate text-neutral-200">{file.path}</span>
                </div>
                <div className="mt-1 flex gap-3 text-xs">
                  {file.insertions > 0 && (
                    <span className="text-green-400">+{file.insertions}</span>
                  )}
                  {file.deletions > 0 && <span className="text-red-400">−{file.deletions}</span>}
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* Diff Content */}
        <div className="flex-1 overflow-auto bg-neutral-950 p-4">
          <pre className="font-mono text-xs text-neutral-300">
            {selectedFile ? filterDiffForFile(diff, selectedFile) : diff}
          </pre>
        </div>
      </div>
    </div>
  )
}

// Helper to extract diff for a specific file
function filterDiffForFile(diff: string, filePath: string): string {
  const lines = diff.split('\n')
  const result: string[] = []
  let inFile = false

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // Check for file header
    if (line.startsWith('diff --git')) {
      inFile = line.includes(filePath)
    }

    if (inFile) {
      result.push(line)
      // Stop at next diff header
      if (i < lines.length - 1 && lines[i + 1].startsWith('diff --git')) {
        break
      }
    }
  }

  return result.length > 0 ? result.join('\n') : diff
}
