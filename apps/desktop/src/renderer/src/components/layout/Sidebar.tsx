import { useQuery } from '@tanstack/react-query'
import { useProjectStore } from '../../store/project'
import { useLayoutStore } from '../../store/layout'
import { useOpenProject } from '../../features/projects/useOpenProject'
import ThreadList from './ThreadList'
import type { Project } from '../../lib/types'

export default function Sidebar() {
  const { sidebarCollapsed, setSidebarCollapsed } = useLayoutStore()
  const { currentProject, setCurrentProject } = useProjectStore()
  const openProject = useOpenProject()

  // Query recent projects
  const { data: projectsData } = useQuery({
    queryKey: ['recent-projects'],
    queryFn: async () => {
      const response = await window.api.invoke('list_recent_projects')
      const result = response as {
        ok: boolean
        result?: { projects: Project[] }
        error?: string
      }
      if (!result.ok || !result.result) {
        throw new Error(result.error || 'Failed to list recent projects')
      }
      return result.result
    },
  })

  const projects = projectsData?.projects || []

  const { data: appVersion } = useQuery({
    queryKey: ['app-version'],
    queryFn: () => window.api.getVersion(),
    staleTime: Infinity,
  })

  const handleOpenProject = async () => {
    try {
      // Open native folder picker
      const path = await window.api.dialog.openFolder()
      if (!path) return // User cancelled

      // Call open_project RPC
      const project = await openProject.mutateAsync(path)
      setCurrentProject(project)
    } catch (err) {
      console.error('Failed to open project:', err)
      // TODO: Show error toast in Step 13 (UX polish)
    }
  }

  if (sidebarCollapsed) {
    return (
      <div className="flex h-full flex-col items-center gap-4 py-4">
        <button
          onClick={() => setSidebarCollapsed(false)}
          className="text-neutral-400 hover:text-neutral-200"
          title="Expand sidebar"
        >
          <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
          </svg>
        </button>
      </div>
    )
  }

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
        <h2 className="text-sm font-semibold">Projects</h2>
        <button
          onClick={() => setSidebarCollapsed(true)}
          className="text-neutral-400 hover:text-neutral-200"
          title="Collapse sidebar"
        >
          <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
      </div>

      {/* Open project button */}
      <div className="border-b border-neutral-800 p-3">
        <button
          onClick={handleOpenProject}
          disabled={openProject.isPending}
          className="w-full rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {openProject.isPending ? 'Opening...' : 'Open Project'}
        </button>
      </div>

      {/* Thread list (only show when project is selected) */}
      {currentProject && <ThreadList />}

      {/* Recent projects list */}
      <div className="flex-1 overflow-y-auto p-2">
        {projects.length === 0 ? (
          <div className="px-3 py-8 text-center text-sm text-neutral-400">
            No recent projects
          </div>
        ) : (
          <div className="space-y-1">
            {projects.map((project) => (
              <button
                key={project.id}
                onClick={() => setCurrentProject(project)}
                className={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
                  currentProject?.id === project.id
                    ? 'bg-neutral-800 text-white'
                    : 'text-neutral-300 hover:bg-neutral-800/50'
                }`}
              >
                <div className="flex items-center gap-2">
                  {project.is_git_repo && (
                    <svg className="h-4 w-4 text-neutral-500" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.17 6.839 9.49.5.092.682-.217.682-.482 0-.237-.008-.866-.013-1.7-2.782.603-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0112 6.836c.85.004 1.705.114 2.504.336 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C19.138 20.167 22 16.418 22 12c0-5.523-4.477-10-10-10z" />
                    </svg>
                  )}
                  <div className="flex-1 overflow-hidden">
                    <div className="truncate font-medium">{project.name}</div>
                    <div className="truncate text-xs text-neutral-500">
                      {project.path}
                    </div>
                    {project.current_branch && (
                      <div className="mt-0.5 text-xs text-neutral-500">
                        {project.current_branch}
                      </div>
                    )}
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="border-t border-neutral-800 px-4 py-2 text-xs text-neutral-500">
        Cox{appVersion ? ` v${appVersion}` : ''}
      </div>
    </div>
  )
}
