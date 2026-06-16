import { useLayoutStore } from '../../store/layout'
import Sidebar from './Sidebar'
import CenterPanel from './CenterPanel'
import BottomPanel from './BottomPanel'

export default function AppShell() {
  const { sidebarCollapsed, bottomPanelVisible } = useLayoutStore()

  return (
    <div className="flex h-screen bg-neutral-950 text-neutral-100">
      {/* Sidebar */}
      <div
        className={`border-r border-neutral-800 transition-all duration-200 ${
          sidebarCollapsed ? 'w-12' : 'w-64'
        }`}
      >
        <Sidebar />
      </div>

      {/* Main content area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Center panel (transcript/timeline) */}
        <div
          className={`flex-1 overflow-hidden ${
            bottomPanelVisible ? 'h-1/2' : 'h-full'
          }`}
        >
          <CenterPanel />
        </div>

        {/* Bottom panel (terminal/diff/logs) */}
        {bottomPanelVisible && (
          <div className="h-1/2 border-t border-neutral-800">
            <BottomPanel />
          </div>
        )}
      </div>
    </div>
  )
}
