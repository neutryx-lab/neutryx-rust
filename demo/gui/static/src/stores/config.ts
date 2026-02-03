import { defineStore } from 'pinia';
import { ref, computed } from 'vue';

export type Theme = 'dark' | 'light' | 'oled';
export type AccentColor = 'indigo' | 'cyan' | 'emerald' | 'amber' | 'rose' | 'violet';
export type LogLevel = 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

export interface AppConfig {
  debugMode: boolean;
  logLevel: LogLevel;
  theme: Theme;
  accentColor: AccentColor;
  reducedMotion: boolean;
  highContrast: boolean;
}

export const useConfigStore = defineStore('config', () => {
  // State
  const debugMode = ref(false);
  const logLevel = ref<LogLevel>('INFO');
  const theme = ref<Theme>('dark');
  const accentColor = ref<AccentColor>('indigo');
  const reducedMotion = ref(false);
  const highContrast = ref(false);
  const isLoaded = ref(false);

  // Getters
  const config = computed<AppConfig>(() => ({
    debugMode: debugMode.value,
    logLevel: logLevel.value,
    theme: theme.value,
    accentColor: accentColor.value,
    reducedMotion: reducedMotion.value,
    highContrast: highContrast.value,
  }));

  // Actions
  function setTheme(newTheme: Theme) {
    theme.value = newTheme;
    applyTheme();
  }

  function setAccentColor(color: AccentColor) {
    accentColor.value = color;
    applyAccentColor();
  }

  function toggleReducedMotion() {
    reducedMotion.value = !reducedMotion.value;
    applyReducedMotion();
  }

  function toggleHighContrast() {
    highContrast.value = !highContrast.value;
    applyHighContrast();
  }

  function setDebugMode(enabled: boolean) {
    debugMode.value = enabled;
  }

  function setLogLevel(level: LogLevel) {
    logLevel.value = level;
  }

  // Apply theme to document body
  function applyTheme() {
    const body = document.body;
    body.classList.remove('light-theme', 'oled-theme');

    if (theme.value === 'light') {
      body.classList.add('light-theme');
    } else if (theme.value === 'oled') {
      body.classList.add('oled-theme');
    }
  }

  // Apply accent color
  function applyAccentColor() {
    document.body.dataset.accent = accentColor.value;
  }

  // Apply reduced motion
  function applyReducedMotion() {
    document.body.classList.toggle('reduce-motion', reducedMotion.value);
  }

  // Apply high contrast
  function applyHighContrast() {
    document.body.classList.toggle('high-contrast', highContrast.value);
  }

  // Initialize from localStorage or defaults
  async function initialize() {
    if (isLoaded.value) return;

    try {
      // Load from localStorage
      const stored = localStorage.getItem('fb-config');
      if (stored) {
        const parsed = JSON.parse(stored) as Partial<AppConfig>;
        if (parsed.theme) theme.value = parsed.theme;
        if (parsed.accentColor) accentColor.value = parsed.accentColor;
        if (parsed.reducedMotion !== undefined) reducedMotion.value = parsed.reducedMotion;
        if (parsed.highContrast !== undefined) highContrast.value = parsed.highContrast;
        if (parsed.debugMode !== undefined) debugMode.value = parsed.debugMode;
        if (parsed.logLevel) logLevel.value = parsed.logLevel;
      }

      // Respect system preference for reduced motion
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
        reducedMotion.value = true;
      }

      // Apply all settings
      applyTheme();
      applyAccentColor();
      applyReducedMotion();
      applyHighContrast();

      isLoaded.value = true;
    } catch (error) {
      console.error('Failed to load config:', error);
    }
  }

  // Save to localStorage
  function persist() {
    try {
      localStorage.setItem('fb-config', JSON.stringify(config.value));
    } catch (error) {
      console.error('Failed to persist config:', error);
    }
  }

  return {
    // State
    debugMode,
    logLevel,
    theme,
    accentColor,
    reducedMotion,
    highContrast,
    isLoaded,
    // Getters
    config,
    // Actions
    setTheme,
    setAccentColor,
    toggleReducedMotion,
    toggleHighContrast,
    setDebugMode,
    setLogLevel,
    initialize,
    persist,
  };
});
