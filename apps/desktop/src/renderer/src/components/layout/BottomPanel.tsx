import { useLayoutStore } from '../../store/layout'

export default function BottomPanel() {
  const { bottomPanelTab, setBottomPanelTab, setBottomPanelVisible } = useLayoutStore()

  return (
    <div className="flex h-full flex-col">
      {/* Tabs header */}
      <div className="flex items-center justify-between border-b border-neutral-800 px-4">
        <div className="flex gap-1">
          <button
            onClick={() => setBottomPanelTab('terminal')}
            className={`px-4 py-2 text-sm font-medium transition-colors ${
              bottomPanelTab === 'terminal'
                ? 'border-b-2 border-blue-500 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            Terminal
          </button>
          <button
            onClick={() => setBottomPanelTab('diff')}
            className={`px-4 py-2 text-sm font-medium transition-colors ${
              bottomPanelTab === 'diff'
                ? 'border-b-2 border-blue-500 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            Diff
          </button>
          <button
            onClick={() => setBottomPanelTab('logs')}
            className={`px-4 py-2 text-sm font-medium transition-colors ${
              bottomPanelTab === 'logs'
                ? 'border-b-2 border-blue-500 text-white'
                : 'text-neutral-400 hover:text-neutral-200'
            }`}
          >
            Logs
          </button>
        </div>
        <button
          onClick={() => setBottomPanelVisible(false)}
          className="p-2 text-neutral-400 hover:text-neutral-200"
          title="Close panel"
        >
          <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      {/* Panel content */}
      <div className="flex-1 overflow-hidden bg-black/30">
        {bottomPanelTab === 'terminal' && <TerminalTab />}
        {bottomPanelTab === 'diff' && <DiffTab />}
        {bottomPanelTab === 'logs' && <LogsTab />}
      </div>
    </div>
  )
}

function TerminalTab() {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="text-center text-neutral-500">
        <svg className="mx-auto h-12 w-12 text-neutral-700" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
        <p className="mt-2 text-sm">Terminal will be available in Milestone 4</p>
      </div>
    </div>
  )
}

function DiffTab() {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="text-center text-neutral-500">
        <svg className="mx-auto h-12 w-12 text-neutral-700" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <p className="mt-2 text-sm">Diff review will be available in Milestone 10</p>
      </div>
    </div>
  )
}

function LogsTab() {
  return (
    <div className="flex h-full flex-col p-4">
      <div className="flex-1 overflow-y-auto rounded-md bg-black/50 p-3 font-mono text-xs">
        <div className="text-neutral-500">
          <div>[{new Date().toISOString()}] Application started</div>
          <div>[{new Date().toISOString()}] Sidecar connected</div>
          <div className="text-neutral-400">[{new Date().toISOString()}] Ready</div>
        </div>
      </div>
    </div>
  )
}
