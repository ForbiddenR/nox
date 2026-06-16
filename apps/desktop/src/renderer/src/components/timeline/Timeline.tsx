import { useEffect, useRef } from 'react'
import type { RunEvent } from '../../lib/types'

interface TimelineProps {
  runId: string
  events: RunEvent[]
}

export default function Timeline({ runId, events }: TimelineProps) {
  const endRef = useRef<HTMLDivElement>(null)

  // Auto-scroll to bottom when new events arrive
  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [events.length])

  const renderEvent = (event: RunEvent) => {
    const payload = event.payload as Record<string, string | Record<string, unknown> | undefined>

    switch (event.event_type) {
      case 'RunStarted':
        return (
          <div className="flex items-center gap-2 text-sm text-blue-400">
            <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <span>Run started</span>
          </div>
        )

      case 'TextDelta':
        return (
          <div className="whitespace-pre-wrap text-sm text-neutral-200">
            {String(payload.text || '')}
          </div>
        )

      case 'ToolCallStarted':
        return (
          <div className="rounded-md border border-neutral-700 bg-neutral-800/50 p-3">
            <div className="flex items-center gap-2 text-sm text-purple-400">
              <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
              <span className="font-medium">{String(payload.tool)}</span>
            </div>
            {payload.args && (
              <pre className="mt-2 text-xs text-neutral-400">
                {JSON.stringify(payload.args, null, 2)}
              </pre>
            )}
          </div>
        )

      case 'ToolCallCompleted':
        return (
          <div className="rounded-md border border-green-700/50 bg-green-900/20 p-2 text-sm text-green-400">
            ✓ {String(payload.result || '')}
          </div>
        )

      case 'RunCompleted':
        return (
          <div className="flex items-center gap-2 text-sm text-green-400">
            <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>Run completed</span>
          </div>
        )

      case 'RunFailed':
        return (
          <div className="flex items-center gap-2 text-sm text-red-400">
            <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <span>Run failed</span>
          </div>
        )

      default:
        return (
          <div className="text-xs text-neutral-500">
            {event.event_type}
          </div>
        )
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {events.map((event) => (
        <div key={event.id} className="group">
          {renderEvent(event)}
          <div className="mt-1 text-xs text-neutral-600 opacity-0 group-hover:opacity-100">
            {new Date(event.created_at).toLocaleTimeString()}
          </div>
        </div>
      ))}
      <div ref={endRef} />
    </div>
  )
}
