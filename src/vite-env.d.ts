/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL?: string
  readonly VITE_WS_URL?: string
  readonly VITE_MOCK_MODE?: string
  readonly VITE_DEMO_MODE?: string
  /** Google Analytics 4 measurement id (G-XXXXXXXXXX). Empty = no analytics and no cookie banner. */
  readonly VITE_GA_MEASUREMENT_ID?: string
}
