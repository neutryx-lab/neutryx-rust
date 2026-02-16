/**
 * Vuetify plugin configuration.
 *
 * Dark theme aligned with the existing CSS variable design system.
 */

import 'vuetify/styles';
import '@mdi/font/css/materialdesignicons.css';

import { createVuetify } from 'vuetify';
import { aliases, mdi } from 'vuetify/iconsets/mdi';
import * as components from 'vuetify/components';
import * as directives from 'vuetify/directives';

const vuetify = createVuetify({
  components,
  directives,
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: { mdi },
  },
  theme: {
    defaultTheme: 'dark',
    themes: {
      dark: {
        dark: true,
        colors: {
          background: '#0f172a',
          surface: '#1e293b',
          'surface-variant': '#334155',
          primary: '#6366f1',
          secondary: '#8b5cf6',
          error: '#ef4444',
          warning: '#f59e0b',
          success: '#22c55e',
          info: '#3b82f6',
          'on-background': '#e2e8f0',
          'on-surface': '#e2e8f0',
        },
      },
      light: {
        dark: false,
        colors: {
          background: '#f8fafc',
          surface: '#ffffff',
          'surface-variant': '#f1f5f9',
          primary: '#4f46e5',
          secondary: '#7c3aed',
          error: '#ef4444',
          warning: '#f59e0b',
          success: '#22c55e',
          info: '#3b82f6',
          'on-background': '#1e293b',
          'on-surface': '#1e293b',
        },
      },
      oled: {
        dark: true,
        colors: {
          background: '#000000',
          surface: '#0a0a0a',
          'surface-variant': '#171717',
          primary: '#818cf8',
          secondary: '#a78bfa',
          error: '#ef4444',
          warning: '#f59e0b',
          success: '#22c55e',
          info: '#3b82f6',
          'on-background': '#fafafa',
          'on-surface': '#fafafa',
        },
      },
    },
  },
  defaults: {
    VCard: { elevation: 1, rounded: 'lg' },
    VBtn: { rounded: 'lg', variant: 'flat' },
    VTextField: { variant: 'outlined', density: 'compact', hideDetails: 'auto' },
    VSelect: { variant: 'outlined', density: 'compact', hideDetails: 'auto' },
    VDataTable: { density: 'compact', hover: true },
  },
});

export default vuetify;
