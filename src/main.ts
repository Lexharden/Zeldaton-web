import { createPinia } from 'pinia'
import { createApp } from 'vue'
import App from './App.vue'
import { vReveal } from './directives/reveal'
import { router } from './router'
import { initAnalytics } from './analytics/analytics'
import './styles/main.css'

createApp(App).use(createPinia()).use(router).directive('reveal', vReveal).mount('#app')
// Google Analytics 4, only if VITE_GA_MEASUREMENT_ID is set and the visitor accepts.
initAnalytics(router)
