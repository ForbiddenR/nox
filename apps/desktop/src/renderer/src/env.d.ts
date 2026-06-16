/// <reference types="vite/client" />

import type { Api } from '../../preload'

declare module '*.css'

declare global {
  interface Window {
    api: Api
  }
}

export {}
