/**
 * FrictionalBank Dashboard - Vue 3 Entry Point
 * Vue 3 + Pinia + Vue Router + Vuetify (Material UI)
 */

import { createApp } from 'vue';
import { createPinia } from 'pinia';
import router from '@/router';
import vuetify from '@/plugins/vuetify';
import App from '@/App.vue';

// Import Tailwind CSS (includes legacy CSS variables)
import '@/assets/tailwind.css';

// Create Vue app
const app = createApp(App);

// Install Pinia
const pinia = createPinia();
app.use(pinia);

// Install Router
app.use(router);

// Install Vuetify
app.use(vuetify);

// Mount app
app.mount('#app');

// Log initialization
console.log('[Vue] FrictionalBank Dashboard initialized');
