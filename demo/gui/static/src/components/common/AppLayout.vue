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
  <div class="app-layout min-h-screen bg-[var(--bg-primary)]">
    <!-- Animated Background -->
    <div class="animated-bg" aria-hidden="true">
      <div class="orb orb-1"></div>
      <div class="orb orb-2"></div>
      <div class="orb orb-3"></div>
      <div class="orb orb-4"></div>
      <div class="orb orb-5"></div>
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
  will-change: transform;
}

/* Primary orb - top right, large */
.orb-1 {
  width: 600px;
  height: 600px;
  background: var(--primary);
  top: -150px;
  right: -100px;
  opacity: 0.15;
  animation: float-1 25s ease-in-out infinite;
}

/* Purple orb - bottom left */
.orb-2 {
  width: 500px;
  height: 500px;
  background: #8b5cf6;
  bottom: -100px;
  left: -80px;
  opacity: 0.12;
  animation: float-2 30s ease-in-out infinite;
}

/* Cyan orb - center */
.orb-3 {
  width: 400px;
  height: 400px;
  background: #06b6d4;
  top: 40%;
  left: 45%;
  opacity: 0.15;
  animation: float-3 20s ease-in-out infinite;
}

/* Emerald orb - top left, smaller */
.orb-4 {
  width: 300px;
  height: 300px;
  background: #10b981;
  top: 10%;
  left: 20%;
  opacity: 0.10;
  animation: float-4 22s ease-in-out infinite;
}

/* Rose orb - bottom right, smallest */
.orb-5 {
  width: 250px;
  height: 250px;
  background: #f43f5e;
  bottom: 15%;
  right: 20%;
  opacity: 0.08;
  animation: float-5 28s ease-in-out infinite;
}

@keyframes float-1 {
  0%, 100% {
    transform: translate(0, 0) scale(1);
  }
  25% {
    transform: translate(-80px, 60px) scale(1.1);
  }
  50% {
    transform: translate(-40px, 120px) scale(0.95);
  }
  75% {
    transform: translate(60px, 40px) scale(1.05);
  }
}

@keyframes float-2 {
  0%, 100% {
    transform: translate(0, 0) scale(1);
  }
  33% {
    transform: translate(100px, -80px) scale(1.15);
  }
  66% {
    transform: translate(50px, -150px) scale(0.9);
  }
}

@keyframes float-3 {
  0%, 100% {
    transform: translate(-50%, -50%) scale(1);
  }
  25% {
    transform: translate(calc(-50% + 80px), calc(-50% - 60px)) scale(1.2);
  }
  50% {
    transform: translate(calc(-50% - 60px), calc(-50% + 80px)) scale(0.85);
  }
  75% {
    transform: translate(calc(-50% + 40px), calc(-50% + 40px)) scale(1.1);
  }
}

@keyframes float-4 {
  0%, 100% {
    transform: translate(0, 0) scale(1);
  }
  30% {
    transform: translate(70px, 100px) scale(1.1);
  }
  60% {
    transform: translate(-50px, 60px) scale(0.9);
  }
  90% {
    transform: translate(30px, -30px) scale(1.05);
  }
}

@keyframes float-5 {
  0%, 100% {
    transform: translate(0, 0) scale(1);
  }
  20% {
    transform: translate(-100px, -50px) scale(1.15);
  }
  40% {
    transform: translate(-60px, 80px) scale(0.9);
  }
  60% {
    transform: translate(50px, 50px) scale(1.1);
  }
  80% {
    transform: translate(80px, -60px) scale(0.95);
  }
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
