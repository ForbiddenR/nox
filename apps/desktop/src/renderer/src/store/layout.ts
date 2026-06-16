import { create } from 'zustand'

interface LayoutState {
  sidebarCollapsed: boolean
  bottomPanelVisible: boolean
  bottomPanelTab: 'terminal' | 'diff' | 'logs'
  setSidebarCollapsed: (collapsed: boolean) => void
  setBottomPanelVisible: (visible: boolean) => void
  setBottomPanelTab: (tab: 'terminal' | 'diff' | 'logs') => void
}

export const useLayoutStore = create<LayoutState>((set) => ({
  sidebarCollapsed: false,
  bottomPanelVisible: true,
  bottomPanelTab: 'terminal',
  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
  setBottomPanelVisible: (visible) => set({ bottomPanelVisible: visible }),
  setBottomPanelTab: (tab) => set({ bottomPanelTab: tab }),
}))
