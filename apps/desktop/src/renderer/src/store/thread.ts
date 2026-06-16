import { create } from 'zustand'
import type { Thread } from '../lib/types'

interface ThreadState {
  currentThread: Thread | null
  setCurrentThread: (thread: Thread | null) => void
}

export const useThreadStore = create<ThreadState>((set) => ({
  currentThread: null,
  setCurrentThread: (thread) => set({ currentThread: thread }),
}))
