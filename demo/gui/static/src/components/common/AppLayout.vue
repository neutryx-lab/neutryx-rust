<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import Sidebar from './Sidebar.vue';
import TopBar from './TopBar.vue';
import type { ViewMeta } from '@/router';

const route = useRoute();

const currentMeta = computed<ViewMeta | undefined>(() => {
  return route.meta as ViewMeta | undefined;
});
</script>

<template>
  <div class="app-layout min-h-screen bg-[var(--background)]">
    <!-- Neural Network Background -->
    <div class="neural-bg" aria-hidden="true">
      <!-- Grid of nodes -->
      <div class="neural-grid"></div>
      <!-- Animated pulses -->
      <div class="pulse pulse-1"></div>
      <div class="pulse pulse-2"></div>
      <div class="pulse pulse-3"></div>
      <div class="pulse pulse-4"></div>
      <!-- Connection lines (SVG) -->
      <svg class="neural-lines" viewBox="0 0 100 100" preserveAspectRatio="none">
        <line class="line line-1" x1="10" y1="20" x2="40" y2="35" />
        <line class="line line-2" x1="40" y1="35" x2="70" y2="25" />
        <line class="line line-3" x1="70" y1="25" x2="90" y2="45" />
        <line class="line line-4" x1="15" y1="60" x2="45" y2="50" />
        <line class="line line-5" x1="45" y1="50" x2="75" y2="65" />
        <line class="line line-6" x1="75" y1="65" x2="95" y2="55" />
        <line class="line line-7" x1="20" y1="85" x2="50" y2="75" />
        <line class="line line-8" x1="50" y1="75" x2="80" y2="90" />
        <line class="line line-9" x1="40" y1="35" x2="45" y2="50" />
        <line class="line line-10" x1="70" y1="25" x2="75" y2="65" />
        <line class="line line-11" x1="45" y1="50" x2="50" y2="75" />
      </svg>
      <!-- Gradient overlay -->
      <div class="neural-gradient"></div>
    </div>

    <!-- Sidebar -->
    <Sidebar />

    <!-- Main Content Area -->
    <div class="main-content ml-[var(--sidebar-width)]">
      <!-- Top Bar -->
      <TopBar
        :title="currentMeta?.title ?? 'Dashboard'"
        :breadcrumb="currentMeta?.breadcrumb ?? 'Overview'"
      />

      <!-- Page Content -->
      <main class="page-content p-6">
        <slot />
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-layout {
  --sidebar-width: 240px;
  position: relative;
  overflow: hidden;
}

/* Neural Network Background */
.neural-bg {
  position: fixed;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  overflow: hidden;
}

.neural-grid {
  position: absolute;
  inset: 0;
  background-image:
    radial-gradient(circle at 1px 1px, rgba(99, 102, 241, 0.15) 1px, transparent 0);
  background-size: 40px 40px;
}

.neural-lines {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.neural-lines .line {
  stroke: rgba(99, 102, 241, 0.2);
  stroke-width: 0.1;
  stroke-linecap: round;
  animation: pulse-line 4s ease-in-out infinite;
}

.neural-lines .line-1 { animation-delay: 0s; }
.neural-lines .line-2 { animation-delay: 0.3s; }
.neural-lines .line-3 { animation-delay: 0.6s; }
.neural-lines .line-4 { animation-delay: 0.9s; }
.neural-lines .line-5 { animation-delay: 1.2s; }
.neural-lines .line-6 { animation-delay: 1.5s; }
.neural-lines .line-7 { animation-delay: 1.8s; }
.neural-lines .line-8 { animation-delay: 2.1s; }
.neural-lines .line-9 { animation-delay: 2.4s; }
.neural-lines .line-10 { animation-delay: 2.7s; }
.neural-lines .line-11 { animation-delay: 3s; }

@keyframes pulse-line {
  0%, 100% { stroke-opacity: 0.2; }
  50% { stroke-opacity: 0.6; }
}

.pulse {
  position: absolute;
  width: 200px;
  height: 200px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(99, 102, 241, 0.3) 0%, transparent 70%);
  animation: pulse-glow 6s ease-in-out infinite;
}

.pulse-1 { top: 10%; left: 20%; animation-delay: 0s; }
.pulse-2 { top: 60%; left: 70%; animation-delay: 1.5s; }
.pulse-3 { top: 30%; left: 80%; animation-delay: 3s; }
.pulse-4 { top: 80%; left: 30%; animation-delay: 4.5s; }

@keyframes pulse-glow {
  0%, 100% { transform: scale(0.8); opacity: 0.3; }
  50% { transform: scale(1.2); opacity: 0.6; }
}

.neural-gradient {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    135deg,
    transparent 0%,
    rgba(15, 23, 42, 0.5) 50%,
    transparent 100%
  );
}

/* Animated Background */
.animated-bg {
  position: fixed;
  inset: 0;
  z-index: 0;
  pointer-events: none;
  overflow: hidden;
}

.orb {
  position: absolute;
  border-radius: 50%;
  filter: blur(80px);
  opacity: 0.15;
  will-change: transform;
}

.orb-1 {
  width: 600px;
  height: 600px;
  background: var(--primary);
  top: -200px;
  right: -100px;
  animation: float-1 25s ease-in-out infinite;
}

.orb-2 {
  width: 500px;
  height: 500px;
  background: #8b5cf6;
  bottom: -150px;
  left: -100px;
  animation: float-2 30s ease-in-out infinite;
}

.orb-3 {
  width: 400px;
  height: 400px;
  background: #06b6d4;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  animation: float-3 20s ease-in-out infinite;
}

@keyframes float-1 {
  0%, 100% { transform: translate(0, 0) scale(1); }
  33% { transform: translate(-50px, 80px) scale(1.1); }
  66% { transform: translate(30px, -40px) scale(0.95); }
}

@keyframes float-2 {
  0%, 100% { transform: translate(0, 0) scale(1); }
  33% { transform: translate(60px, -50px) scale(0.9); }
  66% { transform: translate(-40px, 30px) scale(1.05); }
}

@keyframes float-3 {
  0%, 100% { transform: translate(-50%, -50%) scale(1); }
  50% { transform: translate(-50%, -50%) scale(1.2); }
}

/* Reduce motion for accessibility */
@media (prefers-reduced-motion: reduce) {
  .orb {
    animation: none;
  }
}

.main-content {
  position: relative;
  z-index: 1;
  min-height: 100vh;
  transition: margin-left var(--transition-normal);
}

.page-content {
  padding-top: calc(var(--header-height, 64px) + 1.5rem);
}

/* Responsive: Collapse sidebar on mobile */
@media (max-width: 768px) {
  .app-layout {
    --sidebar-width: 0px;
  }
}
</style>
