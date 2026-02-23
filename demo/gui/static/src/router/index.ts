import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';

// Lazy load views for code splitting
const DashboardView = () => import('@/views/DashboardView.vue');
const PortfolioView = () => import('@/views/PortfolioView.vue');
const RiskView = () => import('@/views/RiskView.vue');
const ExposureView = () => import('@/views/ExposureView.vue');
const ScenariosView = () => import('@/views/ScenariosView.vue');
const MarketDataView = () => import('@/views/MarketDataView.vue');
const CurveBuilderView = () => import('@/views/CurveBuilderView.vue');
const VolcubeBuilderView = () => import('@/views/VolcubeBuilderView.vue');
const PricerView = () => import('@/views/PricerView.vue');
const GraphView = () => import('@/views/GraphView.vue');
const GreeksAnalyserView = () => import('@/views/GreeksAnalyserView.vue');
const XvaEngineView = () => import('@/views/XvaEngineView.vue');
const IncrementalXvaView = () => import('@/views/IncrementalXvaView.vue');

export type ViewId =
  | 'dashboard'
  | 'portfolio'
  | 'risk'
  | 'exposure'
  | 'scenarios'
  | 'market-data'
  | 'curve-builder'
  | 'volcube-builder'
  | 'pricer'
  | 'graph'
  | 'greeks-analyser'
  | 'xva-engine'
  | 'incremental-xva';

export interface ViewMeta extends Record<string | symbol, unknown> {
  title: string;
  breadcrumb: string;
  icon: string;
  navGroup?: 'main' | 'analytics' | 'tools';
}

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/dashboard',
  },
  {
    path: '/dashboard',
    name: 'dashboard',
    component: DashboardView,
    meta: {
      title: 'Dashboard',
      breadcrumb: 'Dashboard',
      icon: 'fa-tachometer-alt',
      navGroup: 'main',
    } as ViewMeta,
  },
  {
    path: '/portfolio',
    name: 'portfolio',
    component: PortfolioView,
    meta: {
      title: 'Portfolio',
      breadcrumb: 'Portfolio',
      icon: 'fa-wallet',
      navGroup: 'main',
    } as ViewMeta,
  },
  {
    path: '/risk',
    name: 'risk',
    component: RiskView,
    meta: {
      title: 'Risk',
      breadcrumb: 'Risk',
      icon: 'fa-chart-pie',
      navGroup: 'main',
    } as ViewMeta,
  },
  {
    path: '/exposure',
    name: 'exposure',
    component: ExposureView,
    meta: {
      title: 'Exposure',
      breadcrumb: 'Exposure',
      icon: 'fa-layer-group',
      navGroup: 'analytics',
    } as ViewMeta,
  },
  {
    path: '/scenarios',
    name: 'scenarios',
    component: ScenariosView,
    meta: {
      title: 'Scenarios',
      breadcrumb: 'Scenarios',
      icon: 'fa-flask',
      navGroup: 'analytics',
    } as ViewMeta,
  },
  {
    path: '/incremental-xva',
    name: 'incremental-xva',
    component: IncrementalXvaView,
    meta: {
      title: 'Incr. XVA',
      breadcrumb: 'Incr. XVA',
      icon: 'fa-balance-scale',
      navGroup: 'analytics',
    } as ViewMeta,
  },
  {
    path: '/market-data',
    name: 'market-data',
    component: MarketDataView,
    meta: {
      title: 'Market Data',
      breadcrumb: 'Market Data',
      icon: 'fa-database',
      navGroup: 'analytics',
    } as ViewMeta,
  },
  {
    path: '/curve-builder',
    name: 'curve-builder',
    component: CurveBuilderView,
    meta: {
      title: 'Curve Builder',
      breadcrumb: 'Curve Builder',
      icon: 'fa-chart-line',
      navGroup: 'tools',
    } as ViewMeta,
  },
  {
    path: '/volcube-builder',
    name: 'volcube-builder',
    component: VolcubeBuilderView,
    meta: {
      title: 'Vol Surface',
      breadcrumb: 'Vol Surface',
      icon: 'fa-cube',
      navGroup: 'tools',
    } as ViewMeta,
  },
  {
    path: '/pricer',
    name: 'pricer',
    component: PricerView,
    meta: {
      title: 'Pricer',
      breadcrumb: 'Pricer',
      icon: 'fa-calculator',
      navGroup: 'tools',
    } as ViewMeta,
  },
  {
    path: '/greeks-analyser',
    name: 'greeks-analyser',
    component: GreeksAnalyserView,
    meta: {
      title: 'Greeks',
      breadcrumb: 'Greeks',
      icon: 'fa-wave-square',
      navGroup: 'tools',
    } as ViewMeta,
  },
  {
    path: '/xva-engine',
    name: 'xva-engine',
    component: XvaEngineView,
    meta: {
      title: 'XVA',
      breadcrumb: 'XVA',
      icon: 'fa-shield-alt',
      navGroup: 'tools',
    } as ViewMeta,
  },
  {
    path: '/graph',
    name: 'graph',
    component: GraphView,
    meta: {
      title: 'Graph',
      breadcrumb: 'Graph',
      icon: 'fa-project-diagram',
      navGroup: 'tools',
    } as ViewMeta,
  },
  {
    path: '/mfm',
    redirect: '/pricer',
  },
  {
    path: '/jy-inflation',
    redirect: '/market-data',
  },
  // Legacy redirect for trade-expansion
  {
    path: '/trade-expansion',
    redirect: '/market-data',
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// Navigation guard for page title
router.beforeEach((to) => {
  const meta = to.meta as ViewMeta | undefined;
  if (meta?.title) {
    document.title = `${meta.title} | Ergodic Bank`;
  }
});

export default router;

// Helper to get routes by nav group
export function getRoutesByGroup(group: ViewMeta['navGroup']): RouteRecordRaw[] {
  return routes.filter((r) => (r.meta as ViewMeta | undefined)?.navGroup === group);
}
