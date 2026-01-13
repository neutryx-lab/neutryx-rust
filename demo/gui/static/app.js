/**
 * FrictionalBank Dashboard - Modern Interactive Application
 * Bento Grid + Particle Animations + Command Palette
 * ========================================================
 */

// ============================================
// Constants & State
// ============================================

const API_BASE = '/api';
const REFRESH_INTERVAL = 30000;

const state = {
    ws: null,
    charts: {},
    portfolio: {
        data: [],
        filteredData: [],
        page: 1,
        pageSize: 10,
        sort: { field: 'id', order: 'asc' },
        filter: '',
        instrumentFilter: '',
        selectedIds: new Set(),
        viewMode: 'table',
        visibleColumns: ['id', 'instrument', 'counterparty', 'maturity', 'notional', 'pv', 'delta', 'vega'],
        advancedFilters: {
            pvMin: null,
            pvMax: null,
            notionalMin: null,
            notionalMax: null,
            maturity: '',
            counterparty: '',
            riskLevel: 'all'
        }
    },
    scenarios: {
        history: [],
        running: false
    },
    particles: [],
    commandPalette: {
        open: false,
        selectedIndex: 0,
        items: []
    }
};

// ============================================
// Utility Functions
// ============================================

const formatNumber = (n, decimals = 2) => 
    n.toLocaleString('en-US', { minimumFractionDigits: decimals, maximumFractionDigits: decimals });

const formatCurrency = (n) => {
    const abs = Math.abs(n);
    let formatted;
    if (abs >= 1e9) formatted = (n / 1e9).toFixed(2) + 'B';
    else if (abs >= 1e6) formatted = (n / 1e6).toFixed(2) + 'M';
    else if (abs >= 1e3) formatted = (n / 1e3).toFixed(2) + 'K';
    else formatted = formatNumber(n);
    return (n >= 0 ? '' : '-') + '$' + formatted.replace('-', '');
};

const formatPercent = (n) => (n >= 0 ? '+' : '') + n.toFixed(1) + '%';

const debounce = (fn, wait) => {
    let timeout;
    return (...args) => {
        clearTimeout(timeout);
        timeout = setTimeout(() => fn(...args), wait);
    };
};

const clamp = (val, min, max) => Math.min(Math.max(val, min), max);

// ============================================
// Particle System
// ============================================

class ParticleSystem {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.particles = [];
        this.mouse = { x: 0, y: 0 };
        this.resize();
        this.init();
        this.animate();
        
        window.addEventListener('resize', () => this.resize());
        window.addEventListener('mousemove', (e) => {
            this.mouse.x = e.clientX;
            this.mouse.y = e.clientY;
        });
    }
    
    resize() {
        this.canvas.width = window.innerWidth;
        this.canvas.height = window.innerHeight;
    }
    
    init() {
        const particleCount = Math.floor((this.canvas.width * this.canvas.height) / 15000);
        this.particles = [];
        
        for (let i = 0; i < particleCount; i++) {
            this.particles.push({
                x: Math.random() * this.canvas.width,
                y: Math.random() * this.canvas.height,
                vx: (Math.random() - 0.5) * 0.5,
                vy: (Math.random() - 0.5) * 0.5,
                size: Math.random() * 2 + 1,
                opacity: Math.random() * 0.5 + 0.2
            });
        }
    }
    
    animate() {
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        
        const isDark = !document.body.classList.contains('light-theme');
        const particleColor = isDark ? '255, 255, 255' : '0, 0, 0';
        const lineColor = isDark ? '99, 102, 241' : '79, 70, 229';
        
        this.particles.forEach((p, i) => {
            // Update position
            p.x += p.vx;
            p.y += p.vy;
            
            // Bounce off edges
            if (p.x < 0 || p.x > this.canvas.width) p.vx *= -1;
            if (p.y < 0 || p.y > this.canvas.height) p.vy *= -1;
            
            // Mouse interaction
            const dx = this.mouse.x - p.x;
            const dy = this.mouse.y - p.y;
            const dist = Math.sqrt(dx * dx + dy * dy);
            
            if (dist < 150) {
                const force = (150 - dist) / 150;
                p.vx -= (dx / dist) * force * 0.02;
                p.vy -= (dy / dist) * force * 0.02;
            }
            
            // Draw particle
            this.ctx.beginPath();
            this.ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
            this.ctx.fillStyle = `rgba(${particleColor}, ${p.opacity * 0.3})`;
            this.ctx.fill();
            
            // Connect nearby particles
            for (let j = i + 1; j < this.particles.length; j++) {
                const p2 = this.particles[j];
                const dx2 = p.x - p2.x;
                const dy2 = p.y - p2.y;
                const dist2 = Math.sqrt(dx2 * dx2 + dy2 * dy2);
                
                if (dist2 < 120) {
                    this.ctx.beginPath();
                    this.ctx.moveTo(p.x, p.y);
                    this.ctx.lineTo(p2.x, p2.y);
                    this.ctx.strokeStyle = `rgba(${lineColor}, ${(1 - dist2 / 120) * 0.15})`;
                    this.ctx.stroke();
                }
            }
        });
        
        requestAnimationFrame(() => this.animate());
    }
}

// ============================================
// Counting Animation
// ============================================

class CountUp {
    constructor(element, endValue, options = {}) {
        this.element = element;
        this.endValue = endValue;
        this.startValue = parseFloat(element.dataset.value) || 0;
        this.duration = options.duration || 1000;
        this.startTime = null;
        this.previousValue = this.startValue;
        
        this.element.dataset.value = endValue;
        this.animate();
    }
    
    animate() {
        if (!this.startTime) this.startTime = performance.now();
        
        const elapsed = performance.now() - this.startTime;
        const progress = Math.min(elapsed / this.duration, 1);
        
        // Easing function (ease-out-expo)
        const easeOutExpo = progress === 1 ? 1 : 1 - Math.pow(2, -10 * progress);
        
        const currentValue = this.startValue + (this.endValue - this.startValue) * easeOutExpo;
        this.element.textContent = formatCurrency(currentValue);
        
        // Update color based on value
        this.element.classList.remove('positive', 'negative');
        if (currentValue > 0) this.element.classList.add('positive');
        else if (currentValue < 0) this.element.classList.add('negative');
        
        if (progress < 1) {
            requestAnimationFrame(() => this.animate());
        } else {
            // Trigger celebration on large positive changes
            if (this.endValue > this.previousValue && this.endValue - this.previousValue > 100000) {
                triggerCelebration();
            }
        }
    }
}

function updateValue(id, value, options = {}) {
    const el = document.getElementById(id);
    if (!el) return;
    
    if (options.animate !== false) {
        new CountUp(el, value, { duration: options.duration || 800 });
    } else {
        el.textContent = formatCurrency(value);
        el.dataset.value = value;
    }
}

// ============================================
// Loading Overlay
// ============================================

function showLoading(message = 'Processing...') {
    const overlay = document.getElementById('loading-overlay');
    if (!overlay) return;
    const span = overlay.querySelector('span');
    if (span) span.textContent = message;
    overlay.classList.add('active');
}

function hideLoading() {
    const overlay = document.getElementById('loading-overlay');
    if (overlay) overlay.classList.remove('active');
}

// ============================================
// Command Palette
// ============================================

class CommandPalette {
    constructor() {
        this.overlay = document.getElementById('command-overlay');
        this.input = document.getElementById('command-input');
        this.results = document.getElementById('command-results');
        this.items = [];
        this.selectedIndex = 0;
        this.isOpen = false;
        
        this.init();
    }
    
    init() {
        // Collect all command items
        this.allItems = Array.from(document.querySelectorAll('.command-item'));
        this.items = [...this.allItems];
        
        // Open trigger
        document.getElementById('open-command')?.addEventListener('click', () => this.open());
        
        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            // Cmd/Ctrl + K to open
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault();
                this.toggle();
            }
            
            // ESC to close
            if (e.key === 'Escape' && this.isOpen) {
                this.close();
            }
            
            // Navigation when open
            if (this.isOpen) {
                if (e.key === 'ArrowDown') {
                    e.preventDefault();
                    this.navigate(1);
                } else if (e.key === 'ArrowUp') {
                    e.preventDefault();
                    this.navigate(-1);
                } else if (e.key === 'Enter') {
                    e.preventDefault();
                    this.executeSelected();
                }
            }
        });
        
        // Click outside to close
        this.overlay.addEventListener('click', (e) => {
            if (e.target === this.overlay) this.close();
        });
        
        // Search input
        this.input.addEventListener('input', () => this.filter());
        
        // Item click
        this.allItems.forEach((item, index) => {
            item.addEventListener('click', () => {
                this.selectedIndex = index;
                this.executeSelected();
            });
            
            item.addEventListener('mouseenter', () => {
                this.selectedIndex = this.items.indexOf(item);
                this.updateSelection();
            });
        });
    }
    
    open() {
        this.isOpen = true;
        this.overlay.classList.add('active');
        this.input.value = '';
        this.input.focus();
        this.filter();
    }
    
    close() {
        this.isOpen = false;
        this.overlay.classList.remove('active');
    }
    
    toggle() {
        this.isOpen ? this.close() : this.open();
    }
    
    filter() {
        const query = this.input.value.toLowerCase();
        
        this.allItems.forEach(item => {
            const text = item.querySelector('span').textContent.toLowerCase();
            const match = text.includes(query);
            item.style.display = match ? 'flex' : 'none';
        });
        
        this.items = this.allItems.filter(item => item.style.display !== 'none');
        this.selectedIndex = 0;
        this.updateSelection();
    }
    
    navigate(direction) {
        this.selectedIndex = (this.selectedIndex + direction + this.items.length) % this.items.length;
        this.updateSelection();
    }
    
    updateSelection() {
        this.allItems.forEach(item => item.classList.remove('selected'));
        if (this.items[this.selectedIndex]) {
            this.items[this.selectedIndex].classList.add('selected');
            this.items[this.selectedIndex].scrollIntoView({ block: 'nearest' });
        }
    }
    
    executeSelected() {
        const item = this.items[this.selectedIndex];
        if (!item) return;
        
        const action = item.dataset.action;
        this.close();
        this.executeAction(action);
    }
    
    executeAction(action) {
        switch (action) {
            case 'recalculate':
                runRecalculation();
                break;
            case 'refresh':
                refreshAllData();
                break;
            case 'export':
                exportReport();
                break;
            case 'goto-dashboard':
                navigateTo('dashboard');
                break;
            case 'goto-portfolio':
                navigateTo('portfolio');
                break;
            case 'goto-risk':
                navigateTo('risk');
                break;
            case 'goto-scenarios':
                navigateTo('scenarios');
                break;
            case 'scenario-stress':
                navigateTo('scenarios');
                setTimeout(() => applyPreset('stress'), 300);
                break;
            case 'scenario-crisis':
                navigateTo('scenarios');
                setTimeout(() => applyPreset('crisis'), 300);
                break;
            case 'toggle-theme':
                toggleTheme();
                break;
        }
    }
}

// ============================================
// Navigation
// ============================================

function navigateTo(viewName) {
    const navItems = document.querySelectorAll('.nav-item');
    const views = document.querySelectorAll('.view');
    
    navItems.forEach(item => {
        item.classList.toggle('active', item.dataset.view === viewName);
    });
    
    views.forEach(view => {
        view.classList.toggle('active', view.id === `${viewName}-view`);
    });
    
    const titles = {
        dashboard: 'Dashboard',
        portfolio: 'Portfolio',
        risk: 'Risk Analysis',
        exposure: 'Exposure Profile',
        scenarios: 'Scenario Analysis',
        analytics: '3D Analytics',
        graph: 'Computation Graph'
    };

    document.getElementById('page-title').textContent = titles[viewName] || viewName;
    document.getElementById('breadcrumb-current').textContent = titles[viewName] || viewName;

    // View-specific actions
    if (viewName === 'exposure') fetchExposure();
    if (viewName === 'risk') fetchRiskMetrics();
    if (viewName === 'analytics') analytics3D.initViewer();
    if (viewName === 'graph') {
        // Initialise graph view and load data
        if (!graphState.svg) {
            initGraphView();
        }
        // Load default graph if not already loaded
        if (!graphManager.getGraph()) {
            graphManager.fetchGraph().catch(e => console.error('Failed to load graph:', e));
        }
    }
}

function initNavigation() {
    document.querySelectorAll('.nav-item').forEach(item => {
        item.addEventListener('click', (e) => {
            e.preventDefault();
            navigateTo(item.dataset.view);
        });
    });
}

// ============================================
// Theme
// ============================================

function toggleTheme() {
    document.body.classList.toggle('light-theme');
    const isLight = document.body.classList.contains('light-theme');
    document.getElementById('theme-toggle').innerHTML = 
        `<i class="fas fa-${isLight ? 'sun' : 'moon'}"></i>`;
    localStorage.setItem('theme', isLight ? 'light' : 'dark');
    updateChartsTheme();
}

function initTheme() {
    const savedTheme = localStorage.getItem('theme');
    if (savedTheme === 'light') {
        document.body.classList.add('light-theme');
        document.getElementById('theme-toggle').innerHTML = '<i class="fas fa-sun"></i>';
    }
    
    document.getElementById('theme-toggle').addEventListener('click', toggleTheme);
}

function updateChartsTheme() {
    const textColor = getComputedStyle(document.body).getPropertyValue('--text-secondary').trim();
    const gridColor = getComputedStyle(document.body).getPropertyValue('--glass-border').trim();
    
    Object.values(state.charts).forEach(chart => {
        if (chart && chart.options) {
            if (chart.options.scales) {
                ['x', 'y'].forEach(axis => {
                    if (chart.options.scales[axis]) {
                        chart.options.scales[axis].ticks.color = textColor;
                        chart.options.scales[axis].grid.color = gridColor;
                    }
                });
            }
            chart.update('none');
        }
    });
}

// ============================================
// API Calls
// ============================================

async function fetchPortfolio() {
    try {
        const response = await fetch(`${API_BASE}/portfolio`);
        const data = await response.json();
        
        updateValue('total-pv', data.total_pv);
        document.getElementById('trade-count').textContent = data.trade_count;
        
        // Enrich data with additional fields for demo
        state.portfolio.data = enrichPortfolioData(data.trades);
        state.portfolio.filteredData = [...state.portfolio.data];
        
        // Populate counterparty filter dropdown
        populateCounterpartyFilter();
        
        renderCurrentView();
        
        updateLastUpdated();
        return data;
    } catch (error) {
        console.error('Failed to fetch portfolio:', error);
        showToast('Failed to fetch portfolio', 'error');
    }
}

function populateCounterpartyFilter() {
    const select = document.getElementById('counterparty-filter');
    if (!select) return;
    
    const counterparties = [...new Set(state.portfolio.data.map(t => t.counterparty).filter(Boolean))];
    select.innerHTML = '<option value="">All Counterparties</option>' + 
        counterparties.map(c => `<option value="${c}">${c}</option>`).join('');
}

async function fetchRiskMetrics() {
    try {
        const response = await fetch(`${API_BASE}/risk`);
        const data = await response.json();
        
        // Dashboard updates
        updateValue('cva', data.cva);
        updateValue('dva', data.dva);
        updateValue('fva', data.fva);
        updateValue('total-xva', data.total_xva);
        
        // Risk view updates
        document.getElementById('risk-cva').textContent = formatCurrency(data.cva);
        document.getElementById('risk-dva').textContent = formatCurrency(data.dva);
        document.getElementById('risk-fva').textContent = formatCurrency(data.fva);
        document.getElementById('risk-total-xva').textContent = formatCurrency(data.total_xva);
        
        // Exposure metrics
        document.getElementById('risk-ee').textContent = formatCurrency(data.ee);
        document.getElementById('risk-epe').textContent = formatCurrency(data.epe);
        document.getElementById('risk-pfe').textContent = formatCurrency(data.pfe);
        
        // Update bars
        const maxXva = Math.max(Math.abs(data.cva), Math.abs(data.dva), Math.abs(data.fva)) * 1.2;
        updateBar('cva-bar', Math.abs(data.cva), maxXva);
        updateBar('dva-bar', Math.abs(data.dva), maxXva);
        updateBar('fva-bar', Math.abs(data.fva), maxXva);
        
        // Update XVA breakdown bar
        const totalAbs = Math.abs(data.cva) + Math.abs(data.dva) + Math.abs(data.fva);
        if (totalAbs > 0) {
            document.getElementById('xva-cva-bar').style.width = (Math.abs(data.cva) / totalAbs * 100) + '%';
            document.getElementById('xva-dva-bar').style.width = (Math.abs(data.dva) / totalAbs * 100) + '%';
            document.getElementById('xva-fva-bar').style.width = (Math.abs(data.fva) / totalAbs * 100) + '%';
        }
        
        // Update ring
        updateRing('fva-ring', Math.abs(data.fva), maxXva);
        
        // Update gauges
        updateGauge('ee', data.ee, data.pfe);
        updateGauge('epe', data.epe, data.pfe);
        updateGauge('pfe', data.pfe, data.pfe);
        
        // Update donut charts
        updateRiskDonut(data);
        updateXvaPie(data);
        
        // Update risk total in donut center
        document.getElementById('risk-total').textContent = formatCurrency(data.total_xva);
        
        updateLastUpdated();
        return data;
    } catch (error) {
        console.error('Failed to fetch risk metrics:', error);
        showToast('Failed to fetch risk metrics', 'error');
    }
}

async function fetchExposure() {
    try {
        const response = await fetch(`${API_BASE}/exposure`);
        const data = await response.json();
        
        updateExposureChart(data.time_series);
        updateMainExposureChart(data.time_series);
        
        // Update legend values
        if (data.time_series.length > 0) {
            const latest = data.time_series[data.time_series.length - 1];
            document.getElementById('legend-pfe').textContent = formatCurrency(latest.pfe);
            document.getElementById('legend-ee').textContent = formatCurrency(latest.ee);
            document.getElementById('legend-epe').textContent = formatCurrency(latest.epe);
            document.getElementById('legend-ene').textContent = formatCurrency(latest.ene);
            
            // Update exposure stats
            const peakPfe = Math.max(...data.time_series.map(d => d.pfe));
            const avgEpe = data.time_series.reduce((sum, d) => sum + d.epe, 0) / data.time_series.length;
            const peakIndex = data.time_series.findIndex(d => d.pfe === peakPfe);
            
            document.getElementById('peak-pfe').textContent = formatCurrency(peakPfe);
            document.getElementById('avg-epe').textContent = formatCurrency(avgEpe);
            document.getElementById('time-to-peak').textContent = data.time_series[peakIndex]?.time.toFixed(1) + 'Y';
            document.getElementById('max-maturity').textContent = data.time_series[data.time_series.length - 1]?.time.toFixed(1) + 'Y';
        }
        
        updateLastUpdated();
        return data;
    } catch (error) {
        console.error('Failed to fetch exposure:', error);
        showToast('Failed to fetch exposure', 'error');
    }
}

// ============================================
// UI Updates
// ============================================

function updateLastUpdated() {
    const now = new Date();
    document.getElementById('last-update').querySelector('span').textContent = now.toLocaleTimeString();
}

function updateBar(id, value, max) {
    const bar = document.getElementById(id);
    if (bar) {
        bar.style.width = clamp((value / max) * 100, 0, 100) + '%';
    }
}

function updateRing(id, value, max) {
    const ring = document.getElementById(id);
    if (ring) {
        const percent = clamp((value / max) * 100, 0, 100);
        const dashoffset = 100 - percent;
        ring.style.strokeDashoffset = dashoffset;
    }
}

function updateGauge(name, value, max) {
    const needle = document.getElementById(`${name}-needle`);
    const path = document.getElementById(`${name}-gauge-path`);
    
    if (needle) {
        const percent = clamp(value / max, 0, 1);
        const angle = -90 + (percent * 180);
        needle.style.transform = `translateX(-50%) rotate(${angle}deg)`;
    }
    
    if (path) {
        const percent = clamp(value / max, 0, 1);
        const dashoffset = 141 * (1 - percent);
        path.style.strokeDashoffset = dashoffset;
    }
}

// ============================================
// Charts
// ============================================

function getChartColors() {
    return {
        pfe: '#f59e0b',
        ee: '#6366f1',
        epe: '#10b981',
        ene: '#ef4444',
        grid: getComputedStyle(document.body).getPropertyValue('--glass-border').trim() || 'rgba(255,255,255,0.08)',
        text: getComputedStyle(document.body).getPropertyValue('--text-secondary').trim() || '#94a3b8'
    };
}

function createLineChartConfig(data, options = {}) {
    const colors = getChartColors();
    const labels = data.map(p => p.time.toFixed(1) + 'Y');
    
    return {
        type: 'line',
        data: {
            labels,
            datasets: [
                {
                    label: 'PFE',
                    data: data.map(p => p.pfe),
                    borderColor: colors.pfe,
                    backgroundColor: colors.pfe + '20',
                    fill: true,
                    tension: 0.4,
                    pointRadius: options.showPoints ? 3 : 0,
                    pointHoverRadius: 6,
                    borderWidth: 2
                },
                {
                    label: 'EE',
                    data: data.map(p => p.ee),
                    borderColor: colors.ee,
                    backgroundColor: colors.ee + '20',
                    fill: true,
                    tension: 0.4,
                    pointRadius: options.showPoints ? 3 : 0,
                    pointHoverRadius: 6,
                    borderWidth: 2
                },
                {
                    label: 'EPE',
                    data: data.map(p => p.epe),
                    borderColor: colors.epe,
                    backgroundColor: colors.epe + '20',
                    fill: true,
                    tension: 0.4,
                    pointRadius: options.showPoints ? 3 : 0,
                    pointHoverRadius: 6,
                    borderWidth: 2
                },
                {
                    label: 'ENE',
                    data: data.map(p => p.ene),
                    borderColor: colors.ene,
                    backgroundColor: colors.ene + '20',
                    fill: true,
                    tension: 0.4,
                    pointRadius: options.showPoints ? 3 : 0,
                    pointHoverRadius: 6,
                    borderWidth: 2
                }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: { mode: 'index', intersect: false },
            plugins: {
                legend: { display: false },
                tooltip: {
                    backgroundColor: 'rgba(20, 20, 30, 0.95)',
                    titleColor: '#f8fafc',
                    bodyColor: '#94a3b8',
                    borderColor: 'rgba(255,255,255,0.1)',
                    borderWidth: 1,
                    padding: 12,
                    cornerRadius: 8,
                    callbacks: {
                        label: ctx => `${ctx.dataset.label}: ${formatCurrency(ctx.parsed.y)}`
                    }
                }
            },
            scales: {
                x: {
                    grid: { color: colors.grid, drawBorder: false },
                    ticks: { color: colors.text, maxTicksLimit: options.compact ? 6 : 12 }
                },
                y: {
                    grid: { color: colors.grid, drawBorder: false },
                    ticks: {
                        color: colors.text,
                        callback: val => formatCurrency(val)
                    }
                }
            },
            animation: { duration: 750, easing: 'easeOutQuart' }
        }
    };
}

function updateExposureChart(data) {
    const ctx = document.getElementById('exposure-chart');
    if (!ctx) return;
    
    if (state.charts.exposure) {
        const config = createLineChartConfig(data, { compact: true });
        state.charts.exposure.data = config.data;
        state.charts.exposure.update('none');
    } else {
        state.charts.exposure = new Chart(ctx, createLineChartConfig(data, { compact: true }));
    }
}

function updateMainExposureChart(data) {
    const ctx = document.getElementById('main-exposure-chart');
    if (!ctx) return;
    
    if (state.charts.mainExposure) {
        const config = createLineChartConfig(data, { showPoints: true });
        state.charts.mainExposure.data = config.data;
        state.charts.mainExposure.update('none');
    } else {
        state.charts.mainExposure = new Chart(ctx, createLineChartConfig(data, { showPoints: true }));
    }
}

function updateRiskDonut(data) {
    const ctx = document.getElementById('risk-donut');
    if (!ctx) return;
    
    const colors = getChartColors();
    const chartData = {
        labels: ['CVA', 'DVA', 'FVA'],
        datasets: [{
            data: [Math.abs(data.cva), Math.abs(data.dva), Math.abs(data.fva)],
            backgroundColor: [colors.ene, colors.epe, colors.pfe],
            borderWidth: 0,
            hoverOffset: 8
        }]
    };
    
    if (state.charts.riskDonut) {
        state.charts.riskDonut.data = chartData;
        state.charts.riskDonut.update('none');
    } else {
        state.charts.riskDonut = new Chart(ctx, {
            type: 'doughnut',
            data: chartData,
            options: {
                responsive: true,
                maintainAspectRatio: false,
                cutout: '70%',
                plugins: {
                    legend: { display: false },
                    tooltip: {
                        backgroundColor: 'rgba(20, 20, 30, 0.95)',
                        callbacks: {
                            label: ctx => `${ctx.label}: ${formatCurrency(ctx.parsed)}`
                        }
                    }
                }
            }
        });
    }
}

function updateXvaPie(data) {
    const ctx = document.getElementById('xva-pie');
    if (!ctx) return;
    
    const colors = getChartColors();
    const chartData = {
        labels: ['CVA', 'DVA', 'FVA'],
        datasets: [{
            data: [Math.abs(data.cva), Math.abs(data.dva), Math.abs(data.fva)],
            backgroundColor: [colors.ene, colors.epe, colors.pfe],
            borderWidth: 0
        }]
    };
    
    if (state.charts.xvaPie) {
        state.charts.xvaPie.data = chartData;
        state.charts.xvaPie.update('none');
    } else {
        state.charts.xvaPie = new Chart(ctx, {
            type: 'pie',
            data: chartData,
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        display: true,
                        position: 'bottom',
                        labels: {
                            color: getChartColors().text,
                            usePointStyle: true,
                            padding: 12
                        }
                    }
                }
            }
        });
    }
}

// ============================================
// Portfolio - Advanced Features
// ============================================

// Generate enriched mock data for demo
function enrichPortfolioData(trades) {
    const counterparties = ['Goldman Sachs', 'Morgan Stanley', 'JP Morgan', 'Citi', 'HSBC', 'Barclays', 'Deutsche Bank', 'BNP Paribas'];
    const ratings = ['AAA', 'AA+', 'AA', 'AA-', 'A+', 'A', 'A-', 'BBB+', 'BBB'];
    
    return trades.map(t => ({
        ...t,
        counterparty: counterparties[Math.floor(Math.random() * counterparties.length)],
        rating: ratings[Math.floor(Math.random() * ratings.length)],
        maturityDate: new Date(Date.now() + Math.random() * 10 * 365 * 24 * 60 * 60 * 1000),
        theta: (Math.random() - 0.5) * 1000,
        rho: (Math.random() - 0.5) * 500,
        cva: -Math.abs(t.pv * (Math.random() * 0.02)),
        pfe: Math.abs(t.notional * (Math.random() * 0.05)),
        riskLevel: Math.random() < 0.3 ? 'low' : Math.random() < 0.7 ? 'medium' : 'high',
        pvHistory: Array.from({ length: 10 }, () => t.pv * (0.9 + Math.random() * 0.2))
    }));
}

function applyAllFilters() {
    let data = [...state.portfolio.data];
    const f = state.portfolio.advancedFilters;
    
    // Text search
    if (state.portfolio.filter) {
        const q = state.portfolio.filter.toLowerCase();
        data = data.filter(t => 
            t.id.toLowerCase().includes(q) || 
            t.instrument.toLowerCase().includes(q) ||
            (t.counterparty && t.counterparty.toLowerCase().includes(q))
        );
    }
    
    // Instrument filter
    if (state.portfolio.instrumentFilter) {
        data = data.filter(t => t.instrument.toLowerCase().includes(state.portfolio.instrumentFilter));
    }
    
    // PV range
    if (f.pvMin !== null) data = data.filter(t => t.pv >= f.pvMin);
    if (f.pvMax !== null) data = data.filter(t => t.pv <= f.pvMax);
    
    // Notional range
    if (f.notionalMin !== null) data = data.filter(t => t.notional >= f.notionalMin);
    if (f.notionalMax !== null) data = data.filter(t => t.notional <= f.notionalMax);
    
    // Maturity filter
    if (f.maturity) {
        const now = new Date();
        data = data.filter(t => {
            if (!t.maturityDate) return true;
            const years = (t.maturityDate - now) / (365 * 24 * 60 * 60 * 1000);
            switch (f.maturity) {
                case '0-1': return years <= 1;
                case '1-5': return years > 1 && years <= 5;
                case '5-10': return years > 5 && years <= 10;
                case '10+': return years > 10;
                default: return true;
            }
        });
    }
    
    // Counterparty filter
    if (f.counterparty) {
        data = data.filter(t => t.counterparty === f.counterparty);
    }
    
    // Risk level
    if (f.riskLevel !== 'all') {
        data = data.filter(t => t.riskLevel === f.riskLevel);
    }
    
    state.portfolio.filteredData = data;
    return data;
}

function renderPortfolioTable() {
    const tbody = document.getElementById('portfolio-body');
    if (!tbody) return;
    
    let data = applyAllFilters();
    
    // Sort
    data.sort((a, b) => {
        let aVal = a[state.portfolio.sort.field];
        let bVal = b[state.portfolio.sort.field];
        if (aVal instanceof Date) { aVal = aVal.getTime(); bVal = bVal.getTime(); }
        else if (typeof aVal === 'string') { aVal = aVal.toLowerCase(); bVal = bVal.toLowerCase(); }
        const mod = state.portfolio.sort.order === 'asc' ? 1 : -1;
        return aVal > bVal ? mod : -mod;
    });
    
    // Update summary stats
    updatePortfolioSummary(data);
    updateFilterCounts();
    
    // Pagination
    const start = (state.portfolio.page - 1) * state.portfolio.pageSize;
    const end = start + state.portfolio.pageSize;
    const pageData = data.slice(start, end);
    const totalPages = Math.ceil(data.length / state.portfolio.pageSize);
    
    // Update pagination info
    document.getElementById('showing-start').textContent = data.length > 0 ? start + 1 : 0;
    document.getElementById('showing-end').textContent = Math.min(end, data.length);
    document.getElementById('total-items').textContent = data.length;
    
    // Update pagination buttons
    renderPaginationButtons(totalPages);
    
    // Update column visibility
    updateColumnVisibility();
    
    // Update sort indicators
    updateSortIndicators();
    
    // Check selected state
    const allSelected = pageData.length > 0 && pageData.every(t => state.portfolio.selectedIds.has(t.id));
    document.getElementById('select-all').checked = allSelected;
    
    // Render rows
    const cols = state.portfolio.visibleColumns;
    tbody.innerHTML = pageData.map(t => {
        const isSelected = state.portfolio.selectedIds.has(t.id);
        const ttm = t.maturityDate ? ((t.maturityDate - new Date()) / (365 * 24 * 60 * 60 * 1000)).toFixed(1) + 'Y' : '-';
        const initials = t.counterparty ? t.counterparty.split(' ').map(w => w[0]).join('').substring(0, 2) : 'XX';
        
        return `
        <tr class="${isSelected ? 'selected' : ''}" data-id="${t.id}">
            <td class="checkbox-col">
                <input type="checkbox" class="row-checkbox" data-id="${t.id}" ${isSelected ? 'checked' : ''}>
            </td>
            <td><code>${t.id}</code></td>
            ${cols.includes('instrument') ? `<td>${t.instrument}</td>` : ''}
            ${cols.includes('counterparty') ? `
                <td>
                    <div class="counterparty-cell">
                        <div class="counterparty-avatar">${initials}</div>
                        <div class="counterparty-info">
                            <span class="counterparty-name">${t.counterparty || '-'}</span>
                            <span class="counterparty-rating">${t.rating || '-'}</span>
                        </div>
                    </div>
                </td>
            ` : ''}
            ${cols.includes('maturity') ? `
                <td>
                    <div class="maturity-cell">
                        <span class="maturity-date">${t.maturityDate ? t.maturityDate.toLocaleDateString() : '-'}</span>
                        <span class="maturity-ttm">${ttm}</span>
                    </div>
                </td>
            ` : ''}
            ${cols.includes('notional') ? `<td>${formatNumber(t.notional, 0)}</td>` : ''}
            ${cols.includes('pv') ? `<td class="${t.pv >= 0 ? 'positive' : 'negative'}">${formatCurrency(t.pv)}</td>` : ''}
            ${cols.includes('delta') ? `<td>${t.delta.toFixed(4)}</td>` : ''}
            ${cols.includes('gamma') ? `<td>${t.gamma.toFixed(6)}</td>` : ''}
            ${cols.includes('vega') ? `<td>${t.vega.toFixed(2)}</td>` : ''}
            ${cols.includes('theta') ? `<td>${t.theta?.toFixed(2) || '-'}</td>` : ''}
            ${cols.includes('rho') ? `<td>${t.rho?.toFixed(2) || '-'}</td>` : ''}
            ${cols.includes('cva') ? `<td class="negative">${t.cva ? formatCurrency(t.cva) : '-'}</td>` : ''}
            ${cols.includes('pfe') ? `<td>${t.pfe ? formatCurrency(t.pfe) : '-'}</td>` : ''}
            <td>
                <span class="risk-indicator ${t.riskLevel}">
                    <i class="fas fa-${t.riskLevel === 'low' ? 'check' : t.riskLevel === 'medium' ? 'minus' : 'exclamation'}"></i>
                    ${t.riskLevel}
                </span>
            </td>
            <td>
                <div class="row-actions">
                    <button class="icon-btn view-trade" data-id="${t.id}" title="View Details"><i class="fas fa-eye"></i></button>
                    <button class="icon-btn analyze-trade" data-id="${t.id}" title="Analyze"><i class="fas fa-chart-line"></i></button>
                </div>
            </td>
        </tr>
    `}).join('');
    
    // Attach row events
    attachRowEvents();
    
    // Show/hide bulk actions
    updateBulkActionsBar();
}

function renderPortfolioGrid() {
    const container = document.getElementById('grid-view');
    if (!container) return;
    
    let data = applyAllFilters();
    
    data.sort((a, b) => {
        let aVal = a[state.portfolio.sort.field];
        let bVal = b[state.portfolio.sort.field];
        if (typeof aVal === 'string') { aVal = aVal.toLowerCase(); bVal = bVal.toLowerCase(); }
        const mod = state.portfolio.sort.order === 'asc' ? 1 : -1;
        return aVal > bVal ? mod : -mod;
    });
    
    const start = (state.portfolio.page - 1) * state.portfolio.pageSize;
    const end = start + state.portfolio.pageSize;
    const pageData = data.slice(start, end);
    
    container.innerHTML = pageData.map(t => {
        const isSelected = state.portfolio.selectedIds.has(t.id);
        return `
        <div class="trade-card ${isSelected ? 'selected' : ''}" data-id="${t.id}">
            <div class="trade-card-header">
                <span class="trade-card-id">${t.id}</span>
                <span class="trade-card-type">${t.instrument}</span>
            </div>
            <div class="trade-card-body">
                <div class="trade-card-stat full-width">
                    <span class="trade-card-label">Present Value</span>
                    <span class="trade-card-value large ${t.pv >= 0 ? 'positive' : 'negative'}">${formatCurrency(t.pv)}</span>
                </div>
                <div class="trade-card-stat">
                    <span class="trade-card-label">Notional</span>
                    <span class="trade-card-value">${formatCurrency(t.notional)}</span>
                </div>
                <div class="trade-card-stat">
                    <span class="trade-card-label">Maturity</span>
                    <span class="trade-card-value">${t.maturityDate?.toLocaleDateString() || '-'}</span>
                </div>
                <div class="trade-card-stat">
                    <span class="trade-card-label">Counterparty</span>
                    <span class="trade-card-value">${t.counterparty || '-'}</span>
                </div>
                <div class="trade-card-stat">
                    <span class="trade-card-label">Risk</span>
                    <span class="risk-indicator ${t.riskLevel}">${t.riskLevel}</span>
                </div>
            </div>
            <div class="trade-card-footer">
                <div class="trade-card-greeks">
                    <div class="greek-mini">
                        <span class="greek-mini-label">Δ</span>
                        <span class="greek-mini-value">${t.delta.toFixed(3)}</span>
                    </div>
                    <div class="greek-mini">
                        <span class="greek-mini-label">Γ</span>
                        <span class="greek-mini-value">${t.gamma.toFixed(4)}</span>
                    </div>
                    <div class="greek-mini">
                        <span class="greek-mini-label">ν</span>
                        <span class="greek-mini-value">${t.vega.toFixed(1)}</span>
                    </div>
                </div>
                <button class="icon-btn view-trade" data-id="${t.id}"><i class="fas fa-arrow-right"></i></button>
            </div>
        </div>
    `}).join('');
    
    // Attach card events
    container.querySelectorAll('.trade-card').forEach(card => {
        card.addEventListener('click', (e) => {
            if (e.target.closest('.view-trade')) return;
            const id = card.dataset.id;
            toggleSelection(id);
        });
        
        card.querySelector('.view-trade')?.addEventListener('click', (e) => {
            e.stopPropagation();
            openTradeDrawer(card.dataset.id);
        });
    });
}

function renderPortfolioHeatmap() {
    const container = document.getElementById('heatmap-container');
    if (!container) return;
    
    let data = applyAllFilters();
    
    container.innerHTML = data.map(t => `
        <div class="heatmap-cell ${t.riskLevel}" data-id="${t.id}" title="${t.id}: ${formatCurrency(t.pv)}">
            ${t.id.slice(-3)}
        </div>
    `).join('');
    
    container.querySelectorAll('.heatmap-cell').forEach(cell => {
        cell.addEventListener('click', () => openTradeDrawer(cell.dataset.id));
    });
}

function updatePortfolioSummary(data) {
    const totalPv = data.reduce((sum, t) => sum + t.pv, 0);
    const avgDelta = data.length > 0 ? data.reduce((sum, t) => sum + t.delta, 0) / data.length : 0;
    const totalVega = data.reduce((sum, t) => sum + t.vega, 0);
    
    document.getElementById('portfolio-total-pv').textContent = formatCurrency(totalPv);
    document.getElementById('portfolio-total-pv').className = `summary-value ${totalPv >= 0 ? 'positive' : 'negative'}`;
    document.getElementById('portfolio-count').textContent = data.length;
    document.getElementById('portfolio-avg-delta').textContent = avgDelta.toFixed(4);
    document.getElementById('portfolio-total-vega').textContent = formatCurrency(totalVega);
    document.getElementById('selected-count').textContent = state.portfolio.selectedIds.size;
}

function updateFilterCounts() {
    const data = state.portfolio.data;
    document.getElementById('count-all').textContent = data.length;
    document.getElementById('count-swap').textContent = data.filter(t => t.instrument.toLowerCase().includes('swap') && !t.instrument.toLowerCase().includes('swaption')).length;
    document.getElementById('count-swaption').textContent = data.filter(t => t.instrument.toLowerCase().includes('swaption')).length;
    document.getElementById('count-cap').textContent = data.filter(t => t.instrument.toLowerCase().includes('cap')).length;
}

function renderPaginationButtons(totalPages) {
    const container = document.getElementById('page-numbers');
    if (!container) return;
    
    const current = state.portfolio.page;
    const buttons = [];
    
    // Always show first page
    if (current > 3) buttons.push(1);
    if (current > 4) buttons.push('...');
    
    // Show pages around current
    for (let i = Math.max(1, current - 2); i <= Math.min(totalPages, current + 2); i++) {
        buttons.push(i);
    }
    
    // Always show last page
    if (current < totalPages - 3) buttons.push('...');
    if (current < totalPages - 2 && totalPages > 1) buttons.push(totalPages);
    
    container.innerHTML = buttons.map(b => 
        b === '...' 
            ? '<span class="page-ellipsis">...</span>'
            : `<button class="page-btn ${b === current ? 'active' : ''}" data-page="${b}">${b}</button>`
    ).join('');
    
    container.querySelectorAll('.page-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            state.portfolio.page = parseInt(btn.dataset.page);
            renderCurrentView();
        });
    });
}

function updateColumnVisibility() {
    const cols = state.portfolio.visibleColumns;
    document.querySelectorAll('[data-col]').forEach(el => {
        const col = el.dataset.col;
        el.style.display = cols.includes(col) ? '' : 'none';
    });
}

function updateSortIndicators() {
    const { field, order } = state.portfolio.sort;
    document.querySelectorAll('.sortable').forEach(th => {
        th.classList.remove('sorted', 'asc', 'desc');
        if (th.dataset.sort === field) {
            th.classList.add('sorted', order);
        }
    });
}

function attachRowEvents() {
    // Checkbox
    document.querySelectorAll('.row-checkbox').forEach(cb => {
        cb.addEventListener('change', () => {
            toggleSelection(cb.dataset.id);
        });
    });
    
    // View button
    document.querySelectorAll('.view-trade').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            openTradeDrawer(btn.dataset.id);
        });
    });
    
    // Analyze button
    document.querySelectorAll('.analyze-trade').forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.stopPropagation();
            showToast(`Analyzing trade ${btn.dataset.id}...`, 'info');
        });
    });
    
    // Row click
    document.querySelectorAll('#portfolio-body tr').forEach(row => {
        row.addEventListener('click', (e) => {
            if (e.target.closest('input, button')) return;
            openTradeDrawer(row.dataset.id);
        });
    });
}

function toggleSelection(id) {
    if (state.portfolio.selectedIds.has(id)) {
        state.portfolio.selectedIds.delete(id);
    } else {
        state.portfolio.selectedIds.add(id);
    }
    renderCurrentView();
}

function selectAll(selected) {
    const data = state.portfolio.filteredData;
    const start = (state.portfolio.page - 1) * state.portfolio.pageSize;
    const end = start + state.portfolio.pageSize;
    const pageData = data.slice(start, end);
    
    pageData.forEach(t => {
        if (selected) {
            state.portfolio.selectedIds.add(t.id);
        } else {
            state.portfolio.selectedIds.delete(t.id);
        }
    });
    
    renderCurrentView();
}

function clearSelection() {
    state.portfolio.selectedIds.clear();
    renderCurrentView();
}

function updateBulkActionsBar() {
    const bar = document.getElementById('bulk-actions');
    if (!bar) return;
    
    const count = state.portfolio.selectedIds.size;
    bar.style.display = count > 0 ? 'flex' : 'none';
    document.getElementById('bulk-count').textContent = count;
}

function renderCurrentView() {
    const mode = state.portfolio.viewMode;
    
    document.getElementById('table-view').style.display = mode === 'table' ? '' : 'none';
    document.getElementById('grid-view').style.display = mode === 'grid' ? '' : 'none';
    document.getElementById('heatmap-view').style.display = mode === 'heatmap' ? '' : 'none';
    
    if (mode === 'table') renderPortfolioTable();
    else if (mode === 'grid') renderPortfolioGrid();
    else if (mode === 'heatmap') renderPortfolioHeatmap();
    
    updateBulkActionsBar();
    document.getElementById('selected-count').textContent = state.portfolio.selectedIds.size;
}

// Trade Detail Drawer
function openTradeDrawer(id) {
    const trade = state.portfolio.data.find(t => t.id === id);
    if (!trade) return;
    
    const drawer = document.getElementById('trade-drawer');
    const body = document.getElementById('drawer-body');
    
    const ttm = trade.maturityDate ? ((trade.maturityDate - new Date()) / (365 * 24 * 60 * 60 * 1000)).toFixed(2) : '-';
    
    body.innerHTML = `
        <div class="drawer-section">
            <h4 class="drawer-section-title">Overview</h4>
            <div class="drawer-grid">
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Trade ID</span>
                    <span class="drawer-stat-value">${trade.id}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Instrument</span>
                    <span class="drawer-stat-value">${trade.instrument}</span>
                </div>
                <div class="drawer-stat full">
                    <span class="drawer-stat-label">Present Value</span>
                    <span class="drawer-stat-value large ${trade.pv >= 0 ? 'positive' : 'negative'}">${formatCurrency(trade.pv)}</span>
                </div>
            </div>
        </div>
        
        <div class="drawer-section">
            <h4 class="drawer-section-title">Counterparty</h4>
            <div class="drawer-grid">
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Name</span>
                    <span class="drawer-stat-value">${trade.counterparty || '-'}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Rating</span>
                    <span class="drawer-stat-value">${trade.rating || '-'}</span>
                </div>
            </div>
        </div>
        
        <div class="drawer-section">
            <h4 class="drawer-section-title">Terms</h4>
            <div class="drawer-grid">
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Notional</span>
                    <span class="drawer-stat-value">${formatCurrency(trade.notional)}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Maturity</span>
                    <span class="drawer-stat-value">${trade.maturityDate?.toLocaleDateString() || '-'}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Time to Maturity</span>
                    <span class="drawer-stat-value">${ttm} years</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Risk Level</span>
                    <span class="risk-indicator ${trade.riskLevel}">${trade.riskLevel}</span>
                </div>
            </div>
        </div>
        
        <div class="drawer-section">
            <h4 class="drawer-section-title">Greeks</h4>
            <div class="drawer-grid">
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Delta (Δ)</span>
                    <span class="drawer-stat-value">${trade.delta.toFixed(6)}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Gamma (Γ)</span>
                    <span class="drawer-stat-value">${trade.gamma.toFixed(8)}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Vega (ν)</span>
                    <span class="drawer-stat-value">${formatCurrency(trade.vega)}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Theta (Θ)</span>
                    <span class="drawer-stat-value">${trade.theta ? formatCurrency(trade.theta) : '-'}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">Rho (ρ)</span>
                    <span class="drawer-stat-value">${trade.rho ? formatCurrency(trade.rho) : '-'}</span>
                </div>
            </div>
        </div>
        
        <div class="drawer-section">
            <h4 class="drawer-section-title">XVA Metrics</h4>
            <div class="drawer-grid">
                <div class="drawer-stat">
                    <span class="drawer-stat-label">CVA</span>
                    <span class="drawer-stat-value negative">${trade.cva ? formatCurrency(trade.cva) : '-'}</span>
                </div>
                <div class="drawer-stat">
                    <span class="drawer-stat-label">PFE</span>
                    <span class="drawer-stat-value">${trade.pfe ? formatCurrency(trade.pfe) : '-'}</span>
                </div>
            </div>
        </div>
    `;
    
    drawer.classList.add('active');
}

function closeTradeDrawer() {
    document.getElementById('trade-drawer').classList.remove('active');
}

// Export Functions
function exportCSV() {
    const data = state.portfolio.filteredData;
    const headers = ['ID', 'Instrument', 'Counterparty', 'Notional', 'PV', 'Delta', 'Gamma', 'Vega', 'Risk Level'];
    const rows = data.map(t => [
        t.id, t.instrument, t.counterparty || '', t.notional, t.pv.toFixed(2),
        t.delta.toFixed(6), t.gamma.toFixed(8), t.vega.toFixed(2), t.riskLevel
    ]);
    
    const csv = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');
    downloadFile(csv, 'portfolio.csv', 'text/csv');
    showToast('Portfolio exported to CSV', 'success');
}

function exportExcel() {
    showToast('Preparing Excel export...', 'info');
    setTimeout(() => {
        showToast('Excel export completed', 'success');
    }, 1000);
}

function downloadFile(content, filename, type) {
    const blob = new Blob([content], { type });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
}

function initPortfolioControls() {
    // Search
    document.getElementById('portfolio-search')?.addEventListener('input', debounce((e) => {
        state.portfolio.filter = e.target.value;
        state.portfolio.page = 1;
        renderCurrentView();
    }, 300));
    
    // Filter chips
    document.querySelectorAll('[data-filter]').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('[data-filter]').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            state.portfolio.instrumentFilter = btn.dataset.filter === 'all' ? '' : btn.dataset.filter;
            state.portfolio.page = 1;
            renderCurrentView();
        });
    });
    
    // Advanced filters toggle
    document.getElementById('toggle-filters')?.addEventListener('click', () => {
        const panel = document.getElementById('advanced-filters');
        panel.style.display = panel.style.display === 'none' ? 'grid' : 'none';
    });
    
    // Apply filters
    document.getElementById('apply-filters')?.addEventListener('click', () => {
        const f = state.portfolio.advancedFilters;
        f.pvMin = parseFloat(document.getElementById('pv-min').value) || null;
        f.pvMax = parseFloat(document.getElementById('pv-max').value) || null;
        f.notionalMin = parseFloat(document.getElementById('notional-min').value) || null;
        f.notionalMax = parseFloat(document.getElementById('notional-max').value) || null;
        f.maturity = document.getElementById('maturity-filter').value;
        f.counterparty = document.getElementById('counterparty-filter').value;
        state.portfolio.page = 1;
        renderCurrentView();
        showToast('Filters applied', 'success');
    });
    
    // Clear filters
    document.getElementById('clear-filters')?.addEventListener('click', () => {
        state.portfolio.advancedFilters = {
            pvMin: null, pvMax: null, notionalMin: null, notionalMax: null,
            maturity: '', counterparty: '', riskLevel: 'all'
        };
        document.getElementById('pv-min').value = '';
        document.getElementById('pv-max').value = '';
        document.getElementById('notional-min').value = '';
        document.getElementById('notional-max').value = '';
        document.getElementById('maturity-filter').value = '';
        document.getElementById('counterparty-filter').value = '';
        document.querySelectorAll('.risk-chip').forEach(c => c.classList.remove('active'));
        document.querySelector('.risk-chip.all').classList.add('active');
        state.portfolio.page = 1;
        renderCurrentView();
        showToast('Filters cleared', 'info');
    });
    
    // Risk level chips
    document.querySelectorAll('.risk-chip').forEach(chip => {
        chip.addEventListener('click', () => {
            document.querySelectorAll('.risk-chip').forEach(c => c.classList.remove('active'));
            chip.classList.add('active');
            state.portfolio.advancedFilters.riskLevel = chip.dataset.risk;
        });
    });
    
    // View mode toggle
    document.querySelectorAll('[data-view-mode]').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('[data-view-mode]').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            state.portfolio.viewMode = btn.dataset.viewMode;
            renderCurrentView();
        });
    });
    
    // Column selector
    document.getElementById('column-toggle')?.addEventListener('click', (e) => {
        e.stopPropagation();
        document.getElementById('column-dropdown').classList.toggle('active');
    });
    
    document.querySelectorAll('.column-option input').forEach(cb => {
        cb.addEventListener('change', () => {
            const col = cb.dataset.col;
            if (cb.checked) {
                state.portfolio.visibleColumns.push(col);
            } else {
                state.portfolio.visibleColumns = state.portfolio.visibleColumns.filter(c => c !== col);
            }
            renderCurrentView();
        });
    });
    
    document.getElementById('reset-columns')?.addEventListener('click', () => {
        state.portfolio.visibleColumns = ['id', 'instrument', 'counterparty', 'maturity', 'notional', 'pv', 'delta', 'vega'];
        document.querySelectorAll('.column-option input').forEach(cb => {
            cb.checked = state.portfolio.visibleColumns.includes(cb.dataset.col);
        });
        renderCurrentView();
    });
    
    // Close dropdown on outside click
    document.addEventListener('click', (e) => {
        if (!e.target.closest('.column-selector-wrapper')) {
            document.getElementById('column-dropdown')?.classList.remove('active');
        }
    });
    
    // Sorting
    document.querySelectorAll('.sortable').forEach(th => {
        th.addEventListener('click', () => {
            const field = th.dataset.sort;
            if (!field) return;
            if (state.portfolio.sort.field === field) {
                state.portfolio.sort.order = state.portfolio.sort.order === 'asc' ? 'desc' : 'asc';
            } else {
                state.portfolio.sort = { field, order: 'asc' };
            }
            renderCurrentView();
        });
    });
    
    // Select all
    document.getElementById('select-all')?.addEventListener('change', (e) => {
        selectAll(e.target.checked);
    });
    
    // Pagination
    document.getElementById('first-page')?.addEventListener('click', () => {
        state.portfolio.page = 1;
        renderCurrentView();
    });
    
    document.getElementById('prev-page')?.addEventListener('click', () => {
        if (state.portfolio.page > 1) {
            state.portfolio.page--;
            renderCurrentView();
        }
    });
    
    document.getElementById('next-page')?.addEventListener('click', () => {
        const totalPages = Math.ceil(state.portfolio.filteredData.length / state.portfolio.pageSize);
        if (state.portfolio.page < totalPages) {
            state.portfolio.page++;
            renderCurrentView();
        }
    });
    
    document.getElementById('last-page')?.addEventListener('click', () => {
        const totalPages = Math.ceil(state.portfolio.filteredData.length / state.portfolio.pageSize);
        state.portfolio.page = totalPages;
        renderCurrentView();
    });
    
    // Page size
    document.getElementById('page-size')?.addEventListener('change', (e) => {
        state.portfolio.pageSize = parseInt(e.target.value);
        state.portfolio.page = 1;
        renderCurrentView();
    });
    
    // Page jump
    document.getElementById('page-jump')?.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            const page = parseInt(e.target.value);
            const totalPages = Math.ceil(state.portfolio.filteredData.length / state.portfolio.pageSize);
            if (page >= 1 && page <= totalPages) {
                state.portfolio.page = page;
                renderCurrentView();
            }
            e.target.value = '';
        }
    });
    
    // Exports
    document.getElementById('export-csv')?.addEventListener('click', exportCSV);
    document.getElementById('export-excel')?.addEventListener('click', exportExcel);
    document.getElementById('print-portfolio')?.addEventListener('click', () => {
        window.print();
        showToast('Printing...', 'info');
    });
    
    // Trade drawer
    document.getElementById('close-drawer')?.addEventListener('click', closeTradeDrawer);
    document.getElementById('drawer-overlay')?.addEventListener('click', closeTradeDrawer);
    
    // Bulk actions
    document.querySelector('.bulk-actions .danger')?.addEventListener('click', clearSelection);
}

// ============================================
// Scenarios
// ============================================

const PRESETS = {
    base: { rate: 0, vol: 0, spread: 0, corr: 0 },
    stress: { rate: -100, vol: 50, spread: 200, corr: -50 },
    crisis: { rate: -200, vol: 100, spread: 500, corr: -100 },
    recovery: { rate: 50, vol: -25, spread: 50, corr: 25 }
};

function applyPreset(name) {
    const preset = PRESETS[name];
    if (!preset) return;
    
    document.getElementById('rate-shock').value = preset.rate;
    document.getElementById('vol-shift').value = preset.vol;
    document.getElementById('spread-shock').value = preset.spread;
    document.getElementById('corr-shift').value = preset.corr;
    
    updateSliderDisplays();
    
    // Update active preset button
    document.querySelectorAll('.preset-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.preset === name);
    });
}

function updateSliderDisplays() {
    document.getElementById('rate-shock-val').textContent = document.getElementById('rate-shock').value + ' bps';
    document.getElementById('vol-shift-val').textContent = document.getElementById('vol-shift').value + '%';
    document.getElementById('spread-shock-val').textContent = document.getElementById('spread-shock').value + ' bps';
    document.getElementById('corr-shift-val').textContent = document.getElementById('corr-shift').value + '%';
}

function initScenarioControls() {
    // Slider displays
    ['rate-shock', 'vol-shift', 'spread-shock', 'corr-shift'].forEach(id => {
        document.getElementById(id)?.addEventListener('input', updateSliderDisplays);
    });
    
    // Presets
    document.querySelectorAll('.preset-btn').forEach(btn => {
        btn.addEventListener('click', () => applyPreset(btn.dataset.preset));
    });
    
    // Run scenario
    document.getElementById('run-scenario')?.addEventListener('click', runScenario);
}

async function runScenario() {
    if (state.scenarios.running) return;
    
    state.scenarios.running = true;
    const statusEl = document.getElementById('scenario-status');
    const resultsEl = document.getElementById('scenario-results');
    
    statusEl.classList.add('running');
    statusEl.querySelector('span:last-child').textContent = 'Running...';
    
    const params = {
        rateShock: parseInt(document.getElementById('rate-shock').value),
        volShift: parseInt(document.getElementById('vol-shift').value),
        spreadShock: parseInt(document.getElementById('spread-shock').value),
        corrShift: parseInt(document.getElementById('corr-shift').value)
    };
    
    try {
        // Simulate
        await new Promise(r => setTimeout(r, 2000));
        
        const base = await fetchRiskMetrics();
        const results = {
            pv: base.total_pv * (1 + params.rateShock / 10000),
            cva: base.cva * (1 + params.spreadShock / 10000),
            dva: base.dva,
            fva: base.fva * (1 + params.volShift / 100)
        };
        
        const pvChange = ((results.pv - base.total_pv) / Math.abs(base.total_pv)) * 100;
        
        resultsEl.innerHTML = `
            <div class="results-grid">
                <div class="result-card">
                    <span class="result-label">Scenario PV</span>
                    <span class="result-value ${results.pv >= 0 ? 'positive' : 'negative'}">${formatCurrency(results.pv)}</span>
                    <span class="result-delta ${pvChange >= 0 ? 'positive' : 'negative'}">
                        <i class="fas fa-arrow-${pvChange >= 0 ? 'up' : 'down'}"></i> ${formatPercent(pvChange)}
                    </span>
                </div>
                <div class="result-card">
                    <span class="result-label">Scenario CVA</span>
                    <span class="result-value negative">${formatCurrency(results.cva)}</span>
                </div>
                <div class="result-card">
                    <span class="result-label">Scenario DVA</span>
                    <span class="result-value positive">${formatCurrency(results.dva)}</span>
                </div>
                <div class="result-card">
                    <span class="result-label">Scenario FVA</span>
                    <span class="result-value negative">${formatCurrency(results.fva)}</span>
                </div>
            </div>
        `;
        
        addToHistory(params, results);
        
        statusEl.classList.remove('running');
        statusEl.classList.add('complete');
        statusEl.querySelector('span:last-child').textContent = 'Complete';
        
        showToast('Scenario analysis completed', 'success');
        triggerCelebration();
        
    } catch (error) {
        statusEl.classList.remove('running');
        statusEl.querySelector('span:last-child').textContent = 'Failed';
        showToast('Scenario failed', 'error');
    }
    
    state.scenarios.running = false;
}

function addToHistory(params, results) {
    const historyEl = document.getElementById('scenario-history');
    
    // Remove empty state
    const empty = historyEl.querySelector('.history-empty');
    if (empty) empty.remove();
    
    const item = document.createElement('div');
    item.className = 'history-item';
    item.innerHTML = `
        <div class="history-info">
            <span class="history-name">Rate: ${params.rateShock}bp, Vol: ${params.volShift}%</span>
            <span class="history-time">${new Date().toLocaleTimeString()}</span>
        </div>
        <span class="history-result ${results.pv >= 0 ? 'positive' : 'negative'}">${formatCurrency(results.pv)}</span>
    `;
    
    historyEl.insertBefore(item, historyEl.firstChild);
    
    while (historyEl.children.length > 10) {
        historyEl.removeChild(historyEl.lastChild);
    }
}

// ============================================
// Actions
// ============================================

async function runRecalculation() {
    showLoading('Recalculating all positions...');
    await Promise.all([fetchPortfolio(), fetchRiskMetrics(), fetchExposure()]);
    hideLoading();
    showToast('All positions recalculated', 'success');
    triggerCelebration();
}

async function refreshAllData() {
    const btn = document.getElementById('refresh-btn');
    btn.querySelector('i').classList.add('fa-spin');
    
    await Promise.all([fetchPortfolio(), fetchRiskMetrics(), fetchExposure()]);
    
    btn.querySelector('i').classList.remove('fa-spin');
    showToast('Data refreshed', 'success');
}

function exportReport() {
    showToast('Preparing report...', 'info');
    setTimeout(() => {
        showToast('Report exported successfully', 'success');
    }, 1500);
}

function initQuickActions() {
    document.querySelectorAll('.action-tile').forEach(tile => {
        tile.addEventListener('click', () => {
            const action = tile.dataset.action;
            switch (action) {
                case 'recalculate': runRecalculation(); break;
                case 'stress-test':
                    navigateTo('scenarios');
                    setTimeout(() => applyPreset('stress'), 300);
                    break;
                case 'export': exportReport(); break;
                case 'what-if':
                    navigateTo('scenarios');
                    break;
            }
        });
    });
    
    document.getElementById('refresh-btn')?.addEventListener('click', refreshAllData);
    
    document.getElementById('run-pricing-btn')?.addEventListener('click', async () => {
        showLoading('Running full pricing...');
        await new Promise(r => setTimeout(r, 2000));
        await Promise.all([fetchPortfolio(), fetchRiskMetrics(), fetchExposure()]);
        hideLoading();
        showToast('Pricing completed', 'success');
        triggerCelebration();
    });
}

// ============================================
// Chart Interactivity
// ============================================

function initChartControls() {
    // Legend toggle
    document.querySelectorAll('.legend-item').forEach(item => {
        item.addEventListener('click', () => {
            item.classList.toggle('active');
            const series = item.dataset.series;
            const chart = state.charts.exposure;
            if (!chart) return;
            
            const idx = ['pfe', 'ee', 'epe', 'ene'].indexOf(series);
            const meta = chart.getDatasetMeta(idx);
            meta.hidden = !item.classList.contains('active');
            chart.update();
        });
    });
    
    // Metric toggle
    document.querySelectorAll('[data-metric]').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('[data-metric]').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            const metric = btn.dataset.metric;
            const chart = state.charts.mainExposure;
            if (!chart) return;
            
            chart.data.datasets.forEach((ds, i) => {
                const meta = chart.getDatasetMeta(i);
                const seriesName = ['pfe', 'ee', 'epe', 'ene'][i];
                meta.hidden = metric !== 'all' && seriesName !== metric;
            });
            chart.update();
        });
    });
    
    // Download
    document.getElementById('download-chart')?.addEventListener('click', () => {
        const chart = state.charts.mainExposure;
        if (!chart) return;
        
        const link = document.createElement('a');
        link.download = 'exposure-chart.png';
        link.href = chart.toBase64Image();
        link.click();
        showToast('Chart downloaded', 'success');
    });
}

// ============================================
// WebSocket
// ============================================

function connectWebSocket() {
    const statusEl = document.getElementById('connection-status');
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    
    state.ws = new WebSocket(`${protocol}//${location.host}${API_BASE}/ws`);
    
    state.ws.onopen = () => {
        statusEl.classList.remove('error');
        statusEl.classList.add('connected');
        statusEl.querySelector('span').textContent = 'Connected';
        showToast('Connected to server', 'success');
    };
    
    state.ws.onclose = () => {
        statusEl.classList.remove('connected');
        statusEl.classList.add('error');
        statusEl.querySelector('span').textContent = 'Disconnected';
        setTimeout(connectWebSocket, 3000);
    };
    
    state.ws.onerror = () => {
        statusEl.classList.remove('connected');
        statusEl.classList.add('error');
        statusEl.querySelector('span').textContent = 'Error';
    };
    
    state.ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleWsMessage(data);
        } catch (e) {
            console.error('WS parse error:', e);
        }
    };
}

function handleWsMessage(data) {
    if (data.type === 'risk') {
        updateValue('total-pv', data.data.total_pv);
        updateValue('cva', data.data.cva);
        updateValue('dva', data.data.dva);
        updateValue('fva', data.data.fva);
    } else if (data.type === 'graph_update') {
        // Task 5.1: Handle graph_update messages via GraphManager
        graphManager.handleGraphUpdate(data);
    }
}

// ============================================
// Tilt Effect
// ============================================

function initTiltEffect() {
    const TILT_INTENSITY = 50; // Higher = more subtle (was 20)
    const TILT_SCALE = 1.01;
    
    document.querySelectorAll('[data-tilt]').forEach(card => {
        card.addEventListener('mousemove', (e) => {
            const rect = card.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            const centerX = rect.width / 2;
            const centerY = rect.height / 2;
            
            const rotateX = (y - centerY) / TILT_INTENSITY;
            const rotateY = (centerX - x) / TILT_INTENSITY;
            
            card.style.transform = `perspective(1000px) rotateX(${rotateX}deg) rotateY(${rotateY}deg) scale(${TILT_SCALE})`;
        });
        
        card.addEventListener('mouseleave', () => {
            card.style.transform = 'perspective(1000px) rotateX(0) rotateY(0) scale(1)';
        });
    });
}

// ============================================
// Enhanced Risk View
// ============================================

function initRiskView() {
    // Initialize counterparty table
    renderCounterpartyTable();
    
    // Initialize risk asset pie chart
    initRiskAssetChart();
    
    // Initialize XVA history chart
    initXvaHistoryChart();
}

function renderCounterpartyTable() {
    const tbody = document.getElementById('counterparty-table-body');
    if (!tbody) return;
    
    const counterparties = [
        { name: 'Goldman Sachs', rating: 'AA-', exposure: 12500000, limit: 50000000, cva: 125000, utilization: 25 },
        { name: 'JP Morgan', rating: 'A+', exposure: 9800000, limit: 40000000, cva: 98000, utilization: 24.5 },
        { name: 'Morgan Stanley', rating: 'A', exposure: 7500000, limit: 30000000, cva: 82500, utilization: 25 },
        { name: 'Barclays', rating: 'A-', exposure: 5200000, limit: 20000000, cva: 65000, utilization: 26 },
        { name: 'Deutsche Bank', rating: 'BBB+', exposure: 3100000, limit: 15000000, cva: 48000, utilization: 20.7 }
    ];
    
    const colors = ['#6366f1', '#06b6d4', '#10b981', '#f59e0b', '#ec4899'];
    
    tbody.innerHTML = counterparties.map((cpty, i) => {
        const status = cpty.utilization > 80 ? 'critical' : cpty.utilization > 50 ? 'warning' : 'healthy';
        const statusIcon = status === 'critical' ? 'exclamation-circle' : status === 'warning' ? 'exclamation-triangle' : 'check-circle';
        const statusText = status === 'critical' ? 'At Risk' : status === 'warning' ? 'Warning' : 'Healthy';
        
        return `
        <tr>
            <td>
                <div class="cpty-name">
                    <div class="cpty-avatar" style="background: ${colors[i]}">${cpty.name.split(' ').map(w => w[0]).join('')}</div>
                    <span>${cpty.name}</span>
                </div>
            </td>
            <td><span class="rating-badge">${cpty.rating}</span></td>
            <td>${formatCurrency(cpty.exposure)}</td>
            <td>${formatCurrency(cpty.limit)}</td>
            <td>
                <div class="utilization-cell">
                    <div class="limit-bar">
                        <div class="limit-fill ${status}" style="width: ${cpty.utilization}%"></div>
                    </div>
                    <span>${cpty.utilization.toFixed(1)}%</span>
                </div>
            </td>
            <td class="negative">${formatCurrency(cpty.cva)}</td>
            <td><span class="status-badge ${status}"><i class="fas fa-${statusIcon}"></i> ${statusText}</span></td>
        </tr>
    `}).join('');
}

function initRiskAssetChart() {
    const ctx = document.getElementById('risk-asset-pie');
    if (!ctx) return;
    
    new Chart(ctx, {
        type: 'doughnut',
        data: {
            labels: ['Interest Rate', 'FX', 'Credit', 'Equity', 'Commodity'],
            datasets: [{
                data: [35, 25, 20, 12, 8],
                backgroundColor: [
                    '#6366f1',
                    '#06b6d4',
                    '#10b981',
                    '#f59e0b',
                    '#ec4899'
                ],
                borderWidth: 0
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            cutout: '65%',
            plugins: {
                legend: {
                    position: 'right',
                    labels: {
                        color: '#94a3b8',
                        usePointStyle: true,
                        padding: 12
                    }
                }
            }
        }
    });
}

function initXvaHistoryChart() {
    const ctx = document.getElementById('xva-history-chart');
    if (!ctx) return;
    
    const labels = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun'];
    
    new Chart(ctx, {
        type: 'line',
        data: {
            labels,
            datasets: [
                {
                    label: 'CVA',
                    data: [1.2, 1.4, 1.3, 1.5, 1.4, 1.35],
                    borderColor: '#ef4444',
                    backgroundColor: 'rgba(239, 68, 68, 0.1)',
                    fill: true,
                    tension: 0.4
                },
                {
                    label: 'DVA',
                    data: [0.3, 0.35, 0.32, 0.38, 0.36, 0.34],
                    borderColor: '#10b981',
                    backgroundColor: 'rgba(16, 185, 129, 0.1)',
                    fill: true,
                    tension: 0.4
                },
                {
                    label: 'FVA',
                    data: [0.5, 0.55, 0.52, 0.58, 0.54, 0.52],
                    borderColor: '#f59e0b',
                    backgroundColor: 'rgba(245, 158, 11, 0.1)',
                    fill: true,
                    tension: 0.4
                }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: {
                    beginAtZero: true,
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                x: {
                    grid: { display: false },
                    ticks: { color: '#64748b' }
                }
            },
            plugins: {
                legend: {
                    labels: { color: '#94a3b8', usePointStyle: true }
                }
            }
        }
    });
}

// ============================================
// Enhanced Exposure View
// ============================================

function initExposureView() {
    // Initialize main exposure chart
    initMainExposureChart();
    
    // Initialize tenor bucket chart
    initTenorBucketChart();
    
    // Initialize exposure distribution chart
    initExposureDistChart();
    
    // Initialize netting set table
    renderNettingSetTable();
    
    // Initialize collateral chart
    initCollateralChart();
    
    // Initialize MC paths chart
    initMCPathsChart();
    
    // Initialize zoom controls
    initZoomControls();
    
    // Initialize interactive legend
    initExposureLegend();
    
    // Initialize toggle buttons
    initExposureToggles();
    
    // Initialize netting set view toggle
    initNettingSetViewToggle();
    
    // Populate summary values
    updateExposureSummary();
}

function initMainExposureChart() {
    const ctx = document.getElementById('main-exposure-chart');
    if (!ctx) return;
    
    const labels = Array.from({length: 60}, (_, i) => `${(i/12).toFixed(1)}Y`);
    
    state.mainExposureChart = new Chart(ctx, {
        type: 'line',
        data: {
            labels,
            datasets: [
                {
                    label: 'PFE',
                    data: labels.map((_, i) => 15 + 10 * Math.sin(i/10) * Math.exp(-i/40)),
                    borderColor: '#6366f1',
                    backgroundColor: 'rgba(99, 102, 241, 0.1)',
                    fill: true,
                    tension: 0.4
                },
                {
                    label: 'EE',
                    data: labels.map((_, i) => 8 + 5 * Math.sin(i/10) * Math.exp(-i/40)),
                    borderColor: '#10b981',
                    backgroundColor: 'rgba(16, 185, 129, 0.1)',
                    fill: true,
                    tension: 0.4
                },
                {
                    label: 'EPE',
                    data: labels.map((_, i) => 6 + 4 * Math.sin(i/8) * Math.exp(-i/50)),
                    borderColor: '#f59e0b',
                    backgroundColor: 'rgba(245, 158, 11, 0.1)',
                    fill: true,
                    tension: 0.4
                },
                {
                    label: 'ENE',
                    data: labels.map((_, i) => -3 - 2 * Math.sin(i/12) * Math.exp(-i/60)),
                    borderColor: '#ef4444',
                    backgroundColor: 'rgba(239, 68, 68, 0.1)',
                    fill: true,
                    tension: 0.4
                }
            ]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            interaction: {
                intersect: false,
                mode: 'index'
            },
            scales: {
                y: {
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                x: {
                    grid: { display: false },
                    ticks: { 
                        color: '#64748b',
                        maxTicksLimit: 10
                    }
                }
            },
            plugins: {
                legend: { display: false },
                tooltip: {
                    callbacks: {
                        label: function(context) {
                            return `${context.dataset.label}: $${context.raw.toFixed(1)}M`;
                        }
                    }
                }
            }
        }
    });
}

function initExposureDistChart() {
    const ctx = document.getElementById('exposure-dist-chart');
    if (!ctx) return;
    
    new Chart(ctx, {
        type: 'bar',
        data: {
            labels: ['<-5M', '-5M to 0', '0 to 5M', '5M to 10M', '10M to 15M', '>15M'],
            datasets: [{
                label: 'Trades',
                data: [5, 12, 25, 18, 8, 3],
                backgroundColor: '#6366f1'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: {
                    beginAtZero: true,
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                x: {
                    grid: { display: false },
                    ticks: { 
                        color: '#64748b',
                        font: { size: 10 }
                    }
                }
            },
            plugins: {
                legend: { display: false }
            }
        }
    });
}

function initExposureToggles() {
    const toggleBtns = document.querySelectorAll('#exposure-view .toggle-btn');
    toggleBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            toggleBtns.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            const metric = btn.dataset.metric;
            updateMainExposureChart(metric);
        });
    });
}

function updateMainExposureChart(metric) {
    if (!state.mainExposureChart) return;
    
    const datasets = state.mainExposureChart.data.datasets;
    
    if (metric === 'all') {
        datasets.forEach(ds => ds.hidden = false);
    } else {
        const metricMap = { pfe: 'PFE', ee: 'EE', epe: 'EPE', ene: 'ENE' };
        datasets.forEach(ds => {
            ds.hidden = ds.label !== metricMap[metric];
        });
    }
    
    state.mainExposureChart.update();
}

function initNettingSetViewToggle() {
    const chips = document.querySelectorAll('[data-ns-view]');
    const tableView = document.getElementById('ns-table-view');
    const chartView = document.getElementById('ns-chart-view');
    
    chips.forEach(chip => {
        chip.addEventListener('click', () => {
            chips.forEach(c => c.classList.remove('active'));
            chip.classList.add('active');
            
            const view = chip.dataset.nsView;
            if (view === 'table') {
                tableView.style.display = 'block';
                chartView.style.display = 'none';
            } else {
                tableView.style.display = 'none';
                chartView.style.display = 'block';
                initNettingSetChart();
            }
        });
    });
}

function initNettingSetChart() {
    const ctx = document.getElementById('netting-set-chart');
    if (!ctx || ctx.chart) return;
    
    ctx.chart = new Chart(ctx, {
        type: 'bar',
        data: {
            labels: ['NS-001', 'NS-002', 'NS-003', 'NS-004', 'NS-005'],
            datasets: [{
                label: 'Gross',
                data: [8.5, 5.2, 3.8, 2.1, 1.2],
                backgroundColor: 'rgba(99, 102, 241, 0.4)'
            }, {
                label: 'Net',
                data: [5.2, 2.8, 1.5, 0.8, 0.45],
                backgroundColor: '#6366f1'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            indexAxis: 'y',
            scales: {
                x: {
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                y: {
                    grid: { display: false },
                    ticks: { color: '#64748b' }
                }
            },
            plugins: {
                legend: {
                    labels: { color: '#94a3b8' }
                }
            }
        }
    });
}

function updateExposureSummary() {
    // Update summary metrics
    const summaryData = {
        'summary-peak-pfe': '$24.5M',
        'summary-avg-epe': '$8.2M',
        'summary-time-peak': '2.3Y',
        'summary-max-mat': '10Y',
        'peak-pfe': '$24.5M',
        'time-to-peak': '2.3Y',
        'avg-epe': '$8.2M',
        'max-maturity': '10Y',
        'ee-1y': '$12.8M',
        'ee-5y': '$6.2M',
        'exp-legend-pfe': '$24.5M',
        'exp-legend-ee': '$12.8M',
        'exp-legend-epe': '$8.2M',
        'exp-legend-ene': '-$3.5M',
        'mc-mean': '$8.5M',
        'mc-95ci': '$2.1M - $18.2M'
    };
    
    Object.entries(summaryData).forEach(([id, value]) => {
        const el = document.getElementById(id);
        if (el) el.textContent = value;
    });
}

function initTenorBucketChart() {
    const ctx = document.getElementById('tenor-bucket-chart');
    if (!ctx) return;
    
    new Chart(ctx, {
        type: 'bar',
        data: {
            labels: ['0-1M', '1-3M', '3-6M', '6-12M', '1-2Y', '2-5Y', '5Y+'],
            datasets: [{
                label: 'EE',
                data: [2.5, 4.2, 6.8, 8.5, 7.2, 5.1, 2.8],
                backgroundColor: '#6366f1'
            }, {
                label: 'PFE',
                data: [3.8, 6.5, 10.2, 12.8, 11.5, 8.2, 4.5],
                backgroundColor: 'rgba(99, 102, 241, 0.4)'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: {
                    beginAtZero: true,
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                x: {
                    grid: { display: false },
                    ticks: { color: '#64748b' }
                }
            },
            plugins: {
                legend: {
                    labels: { color: '#94a3b8' }
                }
            }
        }
    });
}

function renderNettingSetTable() {
    const tbody = document.getElementById('netting-set-body');
    if (!tbody) return;
    
    const nettingSets = [
        { id: 'NS-001', trades: 45, gross: 8500000, net: 5200000, collateral: 2100000 },
        { id: 'NS-002', trades: 32, gross: 5200000, net: 2800000, collateral: 1500000 },
        { id: 'NS-003', trades: 28, gross: 3800000, net: 1500000, collateral: 800000 },
        { id: 'NS-004', trades: 18, gross: 2100000, net: 800000, collateral: 0 },
        { id: 'NS-005', trades: 12, gross: 1200000, net: 450000, collateral: 200000 }
    ];
    
    tbody.innerHTML = nettingSets.map(ns => {
        const benefit = ((ns.gross - ns.net) / ns.gross * 100).toFixed(0);
        return `
            <tr>
                <td><strong>${ns.id}</strong></td>
                <td>${ns.trades}</td>
                <td>${formatCurrency(ns.gross)}</td>
                <td>${formatCurrency(ns.net)}</td>
                <td><span class="netting-benefit">${benefit}%</span></td>
                <td>${ns.collateral > 0 ? formatCurrency(ns.collateral) : '—'}</td>
            </tr>
        `;
    }).join('');
}

function initCollateralChart() {
    const ctx = document.getElementById('collateral-chart');
    if (!ctx) return;
    
    new Chart(ctx, {
        type: 'line',
        data: {
            labels: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun'],
            datasets: [{
                label: 'Posted',
                data: [12, 15, 18, 14, 16, 19],
                borderColor: '#6366f1',
                backgroundColor: 'rgba(99, 102, 241, 0.2)',
                fill: true,
                tension: 0.4
            }, {
                label: 'Received',
                data: [8, 10, 9, 12, 11, 13],
                borderColor: '#10b981',
                backgroundColor: 'rgba(16, 185, 129, 0.2)',
                fill: true,
                tension: 0.4
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: {
                    beginAtZero: true,
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                x: {
                    grid: { display: false },
                    ticks: { color: '#64748b' }
                }
            },
            plugins: {
                legend: {
                    labels: { color: '#94a3b8' }
                }
            }
        }
    });
}

function initMCPathsChart() {
    const ctx = document.getElementById('mc-paths-chart');
    if (!ctx) return;
    
    // Generate random MC paths
    const numPaths = 20;
    const numPoints = 30;
    const datasets = [];
    
    for (let i = 0; i < numPaths; i++) {
        const data = [0];
        for (let j = 1; j < numPoints; j++) {
            const drift = 0.02;
            const vol = 0.15;
            const dt = 1/12;
            const randomShock = (Math.random() - 0.5) * 2;
            data.push(data[j-1] + drift * dt + vol * Math.sqrt(dt) * randomShock);
        }
        
        datasets.push({
            data,
            borderColor: `rgba(99, 102, 241, ${0.1 + Math.random() * 0.2})`,
            borderWidth: 1,
            pointRadius: 0,
            tension: 0.4
        });
    }
    
    // Add mean path
    datasets.push({
        label: 'Mean',
        data: Array.from({length: numPoints}, (_, i) => i * 0.02 / 12),
        borderColor: '#ef4444',
        borderWidth: 2,
        pointRadius: 0,
        tension: 0.4
    });
    
    new Chart(ctx, {
        type: 'line',
        data: {
            labels: Array.from({length: numPoints}, (_, i) => `T+${i}`),
            datasets
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                y: {
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                x: {
                    grid: { display: false },
                    ticks: { 
                        color: '#64748b',
                        maxTicksLimit: 6
                    }
                }
            },
            plugins: {
                legend: { display: false }
            }
        }
    });
}

function initZoomControls() {
    const zoomIn = document.getElementById('zoom-in');
    const zoomOut = document.getElementById('zoom-out');
    const zoomReset = document.getElementById('zoom-reset');
    
    if (!zoomIn || !zoomOut || !zoomReset) return;
    
    // Placeholder - would integrate with chart zoom plugin
    zoomIn.addEventListener('click', () => console.log('Zoom in'));
    zoomOut.addEventListener('click', () => console.log('Zoom out'));
    zoomReset.addEventListener('click', () => console.log('Reset zoom'));
}

function initExposureLegend() {
    const legendToggles = document.querySelectorAll('.legend-toggle');
    
    legendToggles.forEach(toggle => {
        toggle.addEventListener('click', () => {
            toggle.classList.toggle('active');
            // Would update chart visibility
        });
    });
}

// ============================================
// Enhanced Scenarios View
// ============================================

function initEnhancedScenarioControls() {
    // Scenario type buttons
    const typeButtons = document.querySelectorAll('.scenario-type-btn');
    typeButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            typeButtons.forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            const type = btn.dataset.type;
            switchScenarioType(type);
        });
    });
    
    // Additional sliders
    const curveSteepenSlider = document.getElementById('curve-steep');
    const volSkewSlider = document.getElementById('vol-skew');
    const pdMultSlider = document.getElementById('pd-mult');
    
    if (curveSteepenSlider) {
        curveSteepenSlider.addEventListener('input', (e) => {
            document.getElementById('curve-steep-val').textContent = `${e.target.value} bps`;
        });
    }
    
    if (volSkewSlider) {
        volSkewSlider.addEventListener('input', (e) => {
            document.getElementById('vol-skew-val').textContent = `${e.target.value}%`;
        });
    }
    
    if (pdMultSlider) {
        pdMultSlider.addEventListener('input', (e) => {
            document.getElementById('pd-mult-val').textContent = `${(e.target.value / 100).toFixed(1)}×`;
        });
    }
    
    // Historical events
    const eventCards = document.querySelectorAll('.event-card');
    eventCards.forEach(card => {
        card.addEventListener('click', () => {
            eventCards.forEach(c => c.classList.remove('selected'));
            card.classList.add('selected');
            applyHistoricalEvent(card.dataset.event);
        });
    });
    
    // Initialize comparison chart
    initCompareChart();
    
    // Save/Load/Compare buttons
    document.getElementById('save-scenario')?.addEventListener('click', saveScenario);
    document.getElementById('load-scenario')?.addEventListener('click', loadScenario);
    document.getElementById('compare-scenarios')?.addEventListener('click', compareScenarios);
    document.getElementById('clear-history')?.addEventListener('click', clearScenarioHistory);
    document.getElementById('reset-params')?.addEventListener('click', resetParams);
}

function switchScenarioType(type) {
    const parametricControls = document.getElementById('parametric-controls');
    const historicalControls = document.getElementById('historical-controls');
    
    if (!parametricControls || !historicalControls) return;
    
    if (type === 'historical') {
        parametricControls.style.display = 'none';
        historicalControls.style.display = 'block';
    } else {
        parametricControls.style.display = 'block';
        historicalControls.style.display = 'none';
    }
}

function applyHistoricalEvent(event) {
    const presets = {
        '2008-gfc': { rateShock: -150, volShift: 80, spreadShock: 350, corrShift: 60 },
        '2020-covid': { rateShock: -100, volShift: 100, spreadShock: 200, corrShift: 40 },
        '2022-rate-hike': { rateShock: 150, volShift: 30, spreadShock: 50, corrShift: 10 },
        '2011-euro': { rateShock: -50, volShift: 40, spreadShock: 250, corrShift: 30 }
    };
    
    const preset = presets[event];
    if (!preset) return;
    
    // Apply values to sliders
    document.getElementById('rate-shock').value = preset.rateShock;
    document.getElementById('rate-shock-val').textContent = `${preset.rateShock} bps`;
    
    document.getElementById('vol-shift').value = preset.volShift;
    document.getElementById('vol-shift-val').textContent = `${preset.volShift}%`;
    
    document.getElementById('spread-shock').value = preset.spreadShock;
    document.getElementById('spread-shock-val').textContent = `${preset.spreadShock} bps`;
    
    document.getElementById('corr-shift').value = preset.corrShift;
    document.getElementById('corr-shift-val').textContent = `${preset.corrShift}%`;
}

function initCompareChart() {
    const ctx = document.getElementById('compare-chart');
    if (!ctx) return;
    
    new Chart(ctx, {
        type: 'radar',
        data: {
            labels: ['CVA', 'DVA', 'FVA', 'KVA', 'MVA'],
            datasets: [{
                label: 'Base',
                data: [1.2, 0.3, 0.5, 0.8, 0.2],
                borderColor: '#6366f1',
                backgroundColor: 'rgba(99, 102, 241, 0.2)'
            }, {
                label: 'Stress',
                data: [2.1, 0.5, 0.9, 1.2, 0.4],
                borderColor: '#ef4444',
                backgroundColor: 'rgba(239, 68, 68, 0.2)'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                r: {
                    beginAtZero: true,
                    grid: { color: 'rgba(255,255,255,0.1)' },
                    pointLabels: { color: '#94a3b8' },
                    ticks: { display: false }
                }
            },
            plugins: {
                legend: {
                    labels: { color: '#94a3b8', usePointStyle: true }
                }
            }
        }
    });
}

function initImpactChart() {
    const ctx = document.getElementById('impact-chart');
    if (!ctx) return;
    
    state.impactChart = new Chart(ctx, {
        type: 'bar',
        data: {
            labels: ['CVA', 'DVA', 'FVA', 'KVA', 'MVA', 'Total XVA'],
            datasets: [{
                label: 'Base',
                data: [1.2, 0.3, 0.5, 0.8, 0.2, 2.4],
                backgroundColor: '#6366f1'
            }, {
                label: 'Stressed',
                data: [0, 0, 0, 0, 0, 0],
                backgroundColor: '#ef4444'
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            indexAxis: 'y',
            scales: {
                x: {
                    grid: { color: 'rgba(255,255,255,0.05)' },
                    ticks: { color: '#64748b' }
                },
                y: {
                    grid: { display: false },
                    ticks: { color: '#64748b' }
                }
            },
            plugins: {
                legend: {
                    labels: { color: '#94a3b8' }
                }
            }
        }
    });
}

function saveScenario() {
    // Placeholder for save functionality
    alert('Scenario saved');
}

function loadScenario() {
    // Placeholder for load functionality
    alert('Load scenario dialog would open');
}

function compareScenarios() {
    // Placeholder for compare functionality
    alert('Comparison mode enabled');
}

function clearScenarioHistory() {
    const historyContainer = document.getElementById('scenario-history');
    if (historyContainer) {
        historyContainer.innerHTML = `
            <div class="history-empty">
                <i class="fas fa-inbox"></i>
                <span>No scenarios yet</span>
            </div>
        `;
    }
}

function resetParams() {
    document.getElementById('rate-shock').value = 0;
    document.getElementById('rate-shock-val').textContent = '0 bps';
    
    document.getElementById('curve-steep').value = 0;
    document.getElementById('curve-steep-val').textContent = '0 bps';
    
    document.getElementById('vol-shift').value = 0;
    document.getElementById('vol-shift-val').textContent = '0%';
    
    document.getElementById('vol-skew').value = 0;
    document.getElementById('vol-skew-val').textContent = '0%';
    
    document.getElementById('spread-shock').value = 0;
    document.getElementById('spread-shock-val').textContent = '0 bps';
    
    document.getElementById('pd-mult').value = 100;
    document.getElementById('pd-mult-val').textContent = '1.0×';
    
    document.getElementById('corr-shift').value = 0;
    document.getElementById('corr-shift-val').textContent = '0%';
    
    // Reset preset selection
    document.querySelectorAll('.preset-btn').forEach(btn => {
        btn.classList.remove('active');
        if (btn.dataset.preset === 'base') btn.classList.add('active');
    });
}

// Override the original runScenario to show impact
const originalRunScenario = typeof runScenario === 'function' ? runScenario : null;
async function runEnhancedScenario() {
    const runBtn = document.getElementById('run-scenario');
    const statusEl = document.getElementById('scenario-status');
    const resultsEl = document.getElementById('scenario-results');
    const impactSection = document.getElementById('impact-section');
    
    if (!runBtn) return;
    
    runBtn.classList.add('loading');
    statusEl.querySelector('.status-indicator').style.background = 'var(--warning)';
    statusEl.querySelector('span').textContent = 'Running...';
    
    const params = {
        rate_shock: parseFloat(document.getElementById('rate-shock').value),
        vol_shift: parseFloat(document.getElementById('vol-shift').value),
        spread_shock: parseFloat(document.getElementById('spread-shock').value),
        corr_shift: parseFloat(document.getElementById('corr-shift').value)
    };
    
    try {
        const response = await fetch('/api/scenario', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(params)
        });
        
        if (!response.ok) throw new Error('Scenario failed');
        
        const data = await response.json();
        
        // Update results
        resultsEl.innerHTML = `
            <div class="scenario-results-grid">
                <div class="result-card">
                    <span class="result-label">Stressed PV</span>
                    <span class="result-value">${formatCurrency(data.stressed_pv || 0)}</span>
                </div>
                <div class="result-card">
                    <span class="result-label">PV Change</span>
                    <span class="result-value ${(data.pv_change || 0) >= 0 ? 'positive' : 'negative'}">
                        ${formatCurrency(data.pv_change || 0)}
                    </span>
                </div>
                <div class="result-card">
                    <span class="result-label">Stressed CVA</span>
                    <span class="result-value negative">${formatCurrency(data.stressed_cva || 0)}</span>
                </div>
                <div class="result-card">
                    <span class="result-label">Stressed DVA</span>
                    <span class="result-value positive">${formatCurrency(data.stressed_dva || 0)}</span>
                </div>
            </div>
        `;
        
        // Show impact section
        if (impactSection) {
            impactSection.style.display = 'block';
            if (!state.impactChart) {
                initImpactChart();
            }
            // Update impact chart with stressed values
            if (state.impactChart) {
                state.impactChart.data.datasets[1].data = [
                    (data.stressed_cva || 0) / 1000000,
                    (data.stressed_dva || 0) / 1000000,
                    0.9,
                    1.1,
                    0.3,
                    ((data.stressed_cva || 0) + 0.9 + 1.1 + 0.3 - (data.stressed_dva || 0)) / 1000000
                ];
                state.impactChart.update();
            }
        }
        
        // Add to history
        addScenarioToHistory(params, data);
        
        statusEl.querySelector('.status-indicator').style.background = 'var(--success)';
        statusEl.querySelector('span').textContent = 'Complete';
    } catch (e) {
        console.error('Scenario failed:', e);
        resultsEl.innerHTML = `
            <div class="results-placeholder error">
                <div class="placeholder-icon"><i class="fas fa-exclamation-triangle"></i></div>
                <p>Scenario failed</p>
                <span>${e.message}</span>
            </div>
        `;
        statusEl.querySelector('.status-indicator').style.background = 'var(--danger)';
        statusEl.querySelector('span').textContent = 'Failed';
    } finally {
        runBtn.classList.remove('loading');
    }
}

function addScenarioToHistory(params, result) {
    const historyContainer = document.getElementById('scenario-history');
    if (!historyContainer) return;
    
    // Remove empty state
    const emptyState = historyContainer.querySelector('.history-empty');
    if (emptyState) emptyState.remove();
    
    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = 'history-entry';
    entry.innerHTML = `
        <div class="history-time">${timestamp}</div>
        <div class="history-params">
            Rate: ${params.rate_shock}bp, Vol: ${params.vol_shift}%
        </div>
        <div class="history-result ${(result.pv_change || 0) >= 0 ? 'positive' : 'negative'}">
            ${formatCurrency(result.pv_change || 0)}
        </div>
    `;
    
    historyContainer.insertBefore(entry, historyContainer.firstChild);
}

// ============================================
// ============================================
// Toast Notifications
// ============================================

function showToast(typeOrMessage, titleOrType = 'info', message = '', duration = 5000) {
    const container = document.getElementById('toast-container');
    if (!container) return;
    
    // Support both old format: showToast('message', 'type') 
    // and new format: showToast('type', 'title', 'message')
    let type, title, msg;
    
    if (message === '' && typeof titleOrType === 'string' && ['success', 'error', 'warning', 'info'].includes(titleOrType)) {
        // Old format: showToast('message', 'type')
        msg = typeOrMessage;
        type = titleOrType;
        title = type.charAt(0).toUpperCase() + type.slice(1);
    } else {
        // New format: showToast('type', 'title', 'message')
        type = typeOrMessage;
        title = titleOrType;
        msg = message;
    }
    
    const icons = {
        success: 'fa-check',
        warning: 'fa-exclamation-triangle',
        error: 'fa-times-circle',
        info: 'fa-info-circle'
    };
    
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    toast.innerHTML = `
        <div class="toast-icon"><i class="fas ${icons[type] || icons.info}"></i></div>
        <div class="toast-content">
            <div class="toast-title">${title}</div>
            <div class="toast-message">${msg}</div>
        </div>
        <button class="toast-close"><i class="fas fa-times"></i></button>
    `;
    
    container.appendChild(toast);
    
    const closeBtn = toast.querySelector('.toast-close');
    closeBtn.addEventListener('click', () => removeToast(toast));
    
    if (duration > 0) {
        setTimeout(() => removeToast(toast), duration);
    }
    
    return toast;
}

function removeToast(toast) {
    toast.classList.add('toast-out');
    setTimeout(() => toast.remove(), 300);
}

// ============================================
// Alert Panel System
// ============================================

const alertSystem = {
    alerts: [],
    
    init() {
        const alertBtn = document.getElementById('open-alerts');
        const alertPanel = document.getElementById('alert-panel');
        const closeBtn = document.getElementById('close-alerts');
        
        if (alertBtn && alertPanel) {
            alertBtn.addEventListener('click', () => this.toggle());
        }
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this.close());
        }
        
        // Filter buttons
        document.querySelectorAll('.alert-filter').forEach(btn => {
            btn.addEventListener('click', () => this.filter(btn.dataset.filter));
        });
        
        // Generate sample alerts
        this.generateSampleAlerts();
    },
    
    toggle() {
        const panel = document.getElementById('alert-panel');
        panel?.classList.toggle('active');
    },
    
    close() {
        const panel = document.getElementById('alert-panel');
        panel?.classList.remove('active');
    },
    
    add(alert) {
        this.alerts.unshift({
            id: Date.now(),
            timestamp: new Date(),
            read: false,
            ...alert
        });
        this.render();
        this.updateBadge();
        
        if (alert.type === 'critical') {
            showToast('error', alert.title, alert.description);
        }
    },
    
    filter(type) {
        document.querySelectorAll('.alert-filter').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.filter === type);
        });
        this.render(type);
    },
    
    render(filter = 'all') {
        const list = document.getElementById('alert-list');
        if (!list) return;
        
        const filtered = filter === 'all' 
            ? this.alerts 
            : this.alerts.filter(a => a.type === filter);
        
        if (filtered.length === 0) {
            list.innerHTML = '<div class="alert-item"><p style="color:var(--text-muted);text-align:center;">No alerts</p></div>';
            return;
        }
        
        list.innerHTML = filtered.map(alert => `
            <div class="alert-item ${alert.type} ${alert.read ? '' : 'unread'}" data-id="${alert.id}">
                <div class="alert-icon ${alert.type}">
                    <i class="fas fa-${alert.type === 'critical' ? 'exclamation-circle' : alert.type === 'warning' ? 'exclamation-triangle' : 'info-circle'}"></i>
                </div>
                <div class="alert-content">
                    <div class="alert-title">${alert.title}</div>
                    <div class="alert-desc">${alert.description}</div>
                    <div class="alert-time">${this.formatTime(alert.timestamp)}</div>
                </div>
            </div>
        `).join('');
    },
    
    updateBadge() {
        const badge = document.querySelector('.alert-badge');
        const unread = this.alerts.filter(a => !a.read).length;
        if (badge) {
            badge.textContent = unread;
            badge.style.display = unread > 0 ? 'flex' : 'none';
        }
    },
    
    formatTime(date) {
        const now = new Date();
        const diff = now - date;
        if (diff < 60000) return 'Just now';
        if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
        if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
        return date.toLocaleDateString();
    },
    
    generateSampleAlerts() {
        const samples = [
            { type: 'critical', title: 'VaR Limit Breach', description: 'Portfolio VaR exceeded 95% confidence limit' },
            { type: 'warning', title: 'Credit Exposure High', description: 'Counterparty ABC approaching credit limit' },
            { type: 'info', title: 'Market Data Update', description: 'EOD curves loaded successfully' },
            { type: 'warning', title: 'Collateral Call', description: 'Margin call pending for netting set NS-001' },
        ];
        samples.forEach((s, i) => {
            setTimeout(() => this.add(s), i * 500);
        });
    }
};

// ============================================
// Theme Customizer
// ============================================

const themeCustomizer = {
    init() {
        const themeBtn = document.getElementById('open-theme-panel');
        const themePanel = document.getElementById('theme-panel');
        const closeBtn = document.getElementById('close-theme');
        
        if (themeBtn && themePanel) {
            themeBtn.addEventListener('click', () => this.toggle());
        }
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this.close());
        }
        
        // Theme mode buttons
        document.querySelectorAll('.theme-mode-btn').forEach(btn => {
            btn.addEventListener('click', () => this.setMode(btn.dataset.mode));
        });
        
        // Color swatches
        document.querySelectorAll('.color-swatch').forEach(swatch => {
            swatch.addEventListener('click', () => this.setAccent(swatch.dataset.color));
        });
        
        // Toggles
        document.getElementById('high-contrast-toggle')?.addEventListener('change', (e) => {
            document.body.classList.toggle('high-contrast', e.target.checked);
            localStorage.setItem('highContrast', e.target.checked);
        });
        
        document.getElementById('reduce-motion-toggle')?.addEventListener('change', (e) => {
            document.body.classList.toggle('reduce-motion', e.target.checked);
            localStorage.setItem('reduceMotion', e.target.checked);
        });
        
        // Load saved preferences
        this.loadPreferences();
    },
    
    toggle() {
        const panel = document.getElementById('theme-panel');
        panel?.classList.toggle('active');
    },
    
    close() {
        const panel = document.getElementById('theme-panel');
        panel?.classList.remove('active');
    },
    
    setMode(mode) {
        document.querySelectorAll('.theme-mode-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.mode === mode);
        });
        
        document.body.classList.remove('light-theme', 'oled-theme');
        
        if (mode === 'light') {
            document.body.classList.add('light-theme');
        } else if (mode === 'oled') {
            document.body.classList.add('oled-theme');
        } else if (mode === 'auto') {
            const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
            if (!prefersDark) document.body.classList.add('light-theme');
        }
        
        localStorage.setItem('themeMode', mode);
    },
    
    setAccent(color) {
        document.querySelectorAll('.color-swatch').forEach(s => {
            s.classList.toggle('active', s.dataset.color === color);
        });
        
        if (color === 'default') {
            document.body.removeAttribute('data-accent');
        } else {
            document.body.setAttribute('data-accent', color);
        }
        
        localStorage.setItem('accentColor', color);
    },
    
    loadPreferences() {
        const mode = localStorage.getItem('themeMode') || 'dark';
        const accent = localStorage.getItem('accentColor') || 'default';
        const highContrast = localStorage.getItem('highContrast') === 'true';
        const reduceMotion = localStorage.getItem('reduceMotion') === 'true';
        
        this.setMode(mode);
        this.setAccent(accent);
        
        if (highContrast) {
            document.body.classList.add('high-contrast');
            const toggle = document.getElementById('high-contrast-toggle');
            if (toggle) toggle.checked = true;
        }
        
        if (reduceMotion) {
            document.body.classList.add('reduce-motion');
            const toggle = document.getElementById('reduce-motion-toggle');
            if (toggle) toggle.checked = true;
        }
    }
};

// ============================================
// What-If Simulator
// ============================================

const whatIfSimulator = {
    chart: null,
    
    init() {
        const openBtn = document.getElementById('open-whatif-btn');
        const modal = document.getElementById('whatif-modal');
        const closeBtn = document.getElementById('close-whatif');
        const resetBtn = document.getElementById('reset-whatif');
        const applyBtn = document.getElementById('apply-whatif');
        
        if (openBtn) {
            openBtn.addEventListener('click', () => this.open());
        }
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this.close());
        }
        if (resetBtn) {
            resetBtn.addEventListener('click', () => this.close());
        }
        if (applyBtn) {
            applyBtn.addEventListener('click', () => this.runSimulation());
        }
        
        // Close on overlay click
        modal?.addEventListener('click', (e) => {
            if (e.target === modal) this.close();
        });
    },
    
    open() {
        const modal = document.getElementById('whatif-modal');
        modal?.classList.add('active');
        this.initChart();
    },
    
    close() {
        const modal = document.getElementById('whatif-modal');
        modal?.classList.remove('active');
    },
    
    initChart() {
        const ctx = document.getElementById('whatif-chart')?.getContext('2d');
        if (!ctx || this.chart) return;
        
        this.chart = new Chart(ctx, {
            type: 'bar',
            data: {
                labels: ['Current', 'Simulated'],
                datasets: [{
                    label: 'EPE',
                    data: [45.2, 0],
                    backgroundColor: ['rgba(99, 102, 241, 0.7)', 'rgba(16, 185, 129, 0.7)']
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { display: false } },
                scales: {
                    y: { beginAtZero: true, grid: { color: 'rgba(255,255,255,0.05)' } },
                    x: { grid: { display: false } }
                }
            }
        });
    },
    
    async runSimulation() {
        showToast('info', 'Simulation Running', 'Calculating impact...');
        
        // Simulate calculation delay
        await new Promise(resolve => setTimeout(resolve, 1500));
        
        // Generate random impacts
        const impacts = {
            pnl: (Math.random() - 0.5) * 10,
            cva: (Math.random() - 0.4) * 3,
            dva: (Math.random() - 0.3) * 2,
            exp: (Math.random() - 0.3) * 8
        };
        
        // Update impact cards (match HTML IDs)
        const pvEl = document.getElementById('whatif-delta-pv');
        const cvaEl = document.getElementById('whatif-delta-cva');
        const dvaEl = document.getElementById('whatif-delta-dva');
        const expEl = document.getElementById('whatif-delta-exp');
        
        if (pvEl) pvEl.textContent = `$${impacts.pnl >= 0 ? '+' : ''}${impacts.pnl.toFixed(2)}M`;
        if (cvaEl) cvaEl.textContent = `$${impacts.cva >= 0 ? '+' : ''}${impacts.cva.toFixed(2)}M`;
        if (dvaEl) dvaEl.textContent = `$${impacts.dva >= 0 ? '+' : ''}${impacts.dva.toFixed(2)}M`;
        if (expEl) expEl.textContent = `$${impacts.exp >= 0 ? '+' : ''}${impacts.exp.toFixed(1)}M`;
        
        // Update chart
        if (this.chart) {
            this.chart.data.datasets[0].data = [45.2, 45.2 + impacts.exp];
            this.chart.update();
        }
        
        showToast('success', 'Simulation Complete', 'What-if analysis finished');
    }
};

// ============================================
// Report Generator
// ============================================

const reportGenerator = {
    init() {
        const openBtn = document.getElementById('open-report-btn');
        const modal = document.getElementById('report-modal');
        const closeBtn = document.getElementById('close-report');
        const previewBtn = document.getElementById('preview-report');
        const generateBtn = document.getElementById('generate-report');
        
        if (openBtn) {
            openBtn.addEventListener('click', () => this.open());
        }
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this.close());
        }
        if (previewBtn) {
            previewBtn.addEventListener('click', () => this.close());
        }
        if (generateBtn) {
            generateBtn.addEventListener('click', () => this.generate());
        }
        
        // Report type selection
        document.querySelectorAll('.report-type-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                document.querySelectorAll('.report-type-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            });
        });
        
        modal?.addEventListener('click', (e) => {
            if (e.target === modal) this.close();
        });
    },
    
    open() {
        const modal = document.getElementById('report-modal');
        modal?.classList.add('active');
    },
    
    close() {
        const modal = document.getElementById('report-modal');
        modal?.classList.remove('active');
    },
    
    async generate() {
        const format = document.querySelector('input[name="format"]:checked')?.value || 'pdf';
        const type = document.querySelector('.report-type-btn.active')?.dataset.type || 'summary';
        
        showToast('info', 'Generating Report', `Creating ${type} report as ${format.toUpperCase()}...`);
        
        await new Promise(resolve => setTimeout(resolve, 2000));
        
        if (format === 'pdf') {
            this.generatePDF(type);
        } else {
            this.generateExcel(type);
        }
        
        this.close();
    },
    
    generatePDF(type) {
        if (typeof jspdf === 'undefined' || !jspdf.jsPDF) {
            showToast('warning', 'PDF Generation', 'PDF library not loaded. Feature available in production.');
            return;
        }
        
        const { jsPDF } = jspdf;
        const doc = new jsPDF();
        
        doc.setFontSize(20);
        doc.text(`Neutryx ${type.charAt(0).toUpperCase() + type.slice(1)} Report`, 20, 20);
        doc.setFontSize(12);
        doc.text(`Generated: ${new Date().toLocaleString()}`, 20, 30);
        doc.text('Portfolio Summary', 20, 50);
        doc.text(`Total Notional: ${document.querySelector('.stat-value')?.textContent || 'N/A'}`, 20, 60);
        
        doc.save(`neutryx_${type}_report.pdf`);
        showToast('success', 'Report Generated', 'PDF downloaded successfully');
    },
    
    generateExcel(type) {
        if (typeof XLSX === 'undefined') {
            showToast('warning', 'Excel Generation', 'Excel library not loaded. Feature available in production.');
            return;
        }
        
        const wb = XLSX.utils.book_new();
        const data = [
            ['Neutryx Report', '', '', ''],
            ['Type', type, '', ''],
            ['Generated', new Date().toLocaleString(), '', ''],
            ['', '', '', ''],
            ['Metric', 'Value', 'Change', 'Status'],
            ['Total Notional', '$500M', '+2.3%', 'OK'],
            ['CVA', '$12.5M', '-0.8%', 'OK'],
            ['EPE', '$45.2M', '+1.2%', 'Warning'],
        ];
        
        const ws = XLSX.utils.aoa_to_sheet(data);
        XLSX.utils.book_append_sheet(wb, ws, 'Summary');
        XLSX.writeFile(wb, `neutryx_${type}_report.xlsx`);
        showToast('success', 'Report Generated', 'Excel downloaded successfully');
    }
};

// ============================================
// AI Assistant
// ============================================

const aiAssistant = {
    messages: [],
    
    init() {
        const aiBtn = document.getElementById('open-ai-panel');
        const aiPanel = document.getElementById('ai-panel');
        const closeBtn = document.getElementById('close-ai');
        const sendBtn = document.getElementById('ai-send');
        const input = document.getElementById('ai-input');
        
        if (aiBtn && aiPanel) {
            aiBtn.addEventListener('click', () => this.toggle());
        }
        if (closeBtn) {
            closeBtn.addEventListener('click', () => this.close());
        }
        
        if (sendBtn && input) {
            sendBtn.addEventListener('click', () => this.send());
            input.addEventListener('keypress', (e) => {
                if (e.key === 'Enter') this.send();
            });
        }
        
        // Suggestion clicks
        document.querySelectorAll('.ai-suggestions li').forEach(li => {
            li.addEventListener('click', () => {
                if (input) input.value = li.textContent;
                this.send();
            });
        });
    },
    
    toggle() {
        const panel = document.getElementById('ai-panel');
        panel?.classList.toggle('active');
    },
    
    close() {
        const panel = document.getElementById('ai-panel');
        panel?.classList.remove('active');
    },
    
    async send() {
        const input = document.getElementById('ai-input');
        const chat = document.getElementById('ai-chat');
        const query = input?.value.trim();
        
        if (!query || !chat) return;
        
        // Add user message
        this.addMessage('user', query);
        input.value = '';
        
        // Simulate AI thinking
        const thinkingId = this.addMessage('ai', '<i class="fas fa-spinner fa-spin"></i> Analyzing...');
        
        await new Promise(resolve => setTimeout(resolve, 1500));
        
        // Generate response based on query
        const response = this.generateResponse(query);
        
        // Replace thinking message
        const thinkingEl = document.querySelector(`[data-msg-id="${thinkingId}"]`);
        if (thinkingEl) {
            thinkingEl.querySelector('.ai-bubble').innerHTML = response;
        }
    },
    
    addMessage(type, content) {
        const chat = document.getElementById('ai-chat');
        if (!chat) return null;
        
        const id = Date.now();
        const message = document.createElement('div');
        message.className = `ai-message ${type}`;
        message.dataset.msgId = id;
        message.innerHTML = `
            <div class="ai-avatar">
                <i class="fas fa-${type === 'user' ? 'user' : 'robot'}"></i>
            </div>
            <div class="ai-bubble">${content}</div>
        `;
        
        chat.appendChild(message);
        chat.scrollTop = chat.scrollHeight;
        
        return id;
    },
    
    generateResponse(query) {
        const q = query.toLowerCase();
        
        if (q.includes('risk') || q.includes('var')) {
            return `Based on current portfolio analysis:<br><br>
                <strong>VaR (95%)</strong>: $8.2M<br>
                <strong>Expected Shortfall</strong>: $12.1M<br><br>
                The main risk drivers are interest rate swaps (42%) and FX forwards (31%). Consider reducing concentration in EUR/USD positions.`;
        }
        
        if (q.includes('exposure') || q.includes('epe')) {
            return `Current exposure metrics:<br><br>
                <strong>Peak EPE</strong>: $52.3M at 2Y<br>
                <strong>Average EPE</strong>: $45.2M<br><br>
                Netting effectiveness is at 67%. You may benefit from additional netting agreements with top counterparties.`;
        }
        
        if (q.includes('cva') || q.includes('credit')) {
            return `CVA analysis summary:<br><br>
                <strong>Total CVA</strong>: $12.5M<br>
                <strong>Largest contributor</strong>: Counterparty ABC ($3.2M)<br><br>
                Consider credit hedging for top 3 counterparties to reduce CVA by ~25%.`;
        }
        
        if (q.includes('optimize') || q.includes('suggest')) {
            return `Optimization recommendations:<br><br>
                1. <strong>Reduce IR swap duration</strong> - potential VaR reduction of 15%<br>
                2. <strong>Add EUR hedges</strong> - reduce FX exposure by $5M<br>
                3. <strong>Novate trades to CCP</strong> - reduce counterparty risk<br><br>
                Would you like detailed analysis on any of these?`;
        }
        
        return `I can help you analyze:<br><br>
            • <strong>Risk metrics</strong> - VaR, ES, sensitivities<br>
            • <strong>Exposure profiles</strong> - EPE, PFE, netting<br>
            • <strong>XVA analysis</strong> - CVA, DVA, FVA<br>
            • <strong>Optimization</strong> - hedge recommendations<br><br>
            What would you like to explore?`;
    }
};

// ============================================
// 3D Analytics (Three.js)
// ============================================

const analytics3D = {
    scene: null,
    camera: null,
    renderer: null,
    controls: null,
    
    init() {
        if (typeof THREE === 'undefined') {
            console.log('Three.js not loaded, 3D analytics disabled');
            return;
        }
        
        this.initCorrelationHeatmap();
        this.initSankeyDiagram();
        this.initDistributionChart();
    },
    
    initViewer() {
        const container = document.getElementById('three-container');
        if (!container || this.renderer) return;
        
        // Scene
        this.scene = new THREE.Scene();
        this.scene.background = new THREE.Color(0x0a0a12);
        
        // Camera
        this.camera = new THREE.PerspectiveCamera(
            60, container.clientWidth / container.clientHeight, 0.1, 1000
        );
        this.camera.position.set(5, 5, 5);
        
        // Renderer
        this.renderer = new THREE.WebGLRenderer({ antialias: true });
        this.renderer.setSize(container.clientWidth, container.clientHeight);
        container.appendChild(this.renderer.domElement);
        
        // Lights
        const ambientLight = new THREE.AmbientLight(0x404040, 0.5);
        this.scene.add(ambientLight);
        
        const directionalLight = new THREE.DirectionalLight(0xffffff, 1);
        directionalLight.position.set(5, 10, 5);
        this.scene.add(directionalLight);
        
        // Create vol surface
        this.createVolatilitySurface();
        
        // Animation
        const animate = () => {
            requestAnimationFrame(animate);
            this.renderer.render(this.scene, this.camera);
        };
        animate();
        
        // Resize handler
        window.addEventListener('resize', () => {
            if (!container) return;
            this.camera.aspect = container.clientWidth / container.clientHeight;
            this.camera.updateProjectionMatrix();
            this.renderer.setSize(container.clientWidth, container.clientHeight);
        });
    },
    
    createVolatilitySurface() {
        const geometry = new THREE.PlaneGeometry(4, 4, 32, 32);
        const positions = geometry.attributes.position;
        
        // Modify vertices to create surface
        for (let i = 0; i < positions.count; i++) {
            const x = positions.getX(i);
            const y = positions.getY(i);
            const z = Math.sin(x * 2) * Math.cos(y * 2) * 0.5 + 
                      Math.exp(-0.5 * (x * x + y * y)) * 0.3;
            positions.setZ(i, z);
        }
        
        geometry.computeVertexNormals();
        
        const material = new THREE.MeshPhongMaterial({
            color: 0x6366f1,
            side: THREE.DoubleSide,
            flatShading: true,
            transparent: true,
            opacity: 0.9
        });
        
        const mesh = new THREE.Mesh(geometry, material);
        mesh.rotation.x = -Math.PI / 2;
        this.scene.add(mesh);
        
        // Grid helper
        const gridHelper = new THREE.GridHelper(4, 10, 0x444444, 0x222222);
        gridHelper.position.y = -0.5;
        this.scene.add(gridHelper);
    },
    
    initCorrelationHeatmap() {
        const container = document.querySelector('.heatmap');
        if (!container) return;
        
        const assets = ['EUR/USD', 'GBP/USD', 'USD/JPY', 'IR-EUR', 'IR-USD'];
        const correlations = [
            [1.0, 0.7, -0.3, 0.2, 0.1],
            [0.7, 1.0, -0.2, 0.3, 0.2],
            [-0.3, -0.2, 1.0, -0.1, 0.4],
            [0.2, 0.3, -0.1, 1.0, 0.8],
            [0.1, 0.2, 0.4, 0.8, 1.0]
        ];
        
        const cellSize = Math.min(container.clientWidth, container.clientHeight) / assets.length;
        
        const html = assets.map((rowAsset, i) => 
            `<div style="display:flex;">` +
            correlations[i].map((val, j) => {
                const color = val > 0 
                    ? `rgba(99, 102, 241, ${Math.abs(val)})` 
                    : `rgba(239, 68, 68, ${Math.abs(val)})`;
                return `<div style="width:${cellSize}px;height:${cellSize}px;background:${color};display:flex;align-items:center;justify-content:center;font-size:0.6rem;color:white;" title="${rowAsset} vs ${assets[j]}">${val.toFixed(1)}</div>`;
            }).join('') +
            `</div>`
        ).join('');
        
        container.innerHTML = html;
    },
    
    initSankeyDiagram() {
        const container = document.getElementById('sankey-container');
        if (!container || typeof d3 === 'undefined') return;
        
        const width = container.clientWidth;
        const height = container.clientHeight;
        
        const svg = d3.select(container).append('svg')
            .attr('width', width)
            .attr('height', height);
        
        // Simplified flow visualization
        const data = {
            nodes: [
                { name: 'Swaps' }, { name: 'FX' }, { name: 'Options' },
                { name: 'EPE' }, { name: 'CVA' }
            ],
            links: [
                { source: 0, target: 3, value: 30 },
                { source: 0, target: 4, value: 15 },
                { source: 1, target: 3, value: 20 },
                { source: 1, target: 4, value: 10 },
                { source: 2, target: 3, value: 15 },
                { source: 2, target: 4, value: 5 }
            ]
        };
        
        // Draw simplified flow lines
        const colors = ['#6366f1', '#22c55e', '#f59e0b'];
        data.links.forEach((link, i) => {
            const sourceY = 20 + link.source * 40;
            const targetY = 20 + (link.target - 3) * 60;
            
            svg.append('path')
                .attr('d', `M 30 ${sourceY} C 80 ${sourceY}, ${width - 80} ${targetY}, ${width - 30} ${targetY}`)
                .attr('fill', 'none')
                .attr('stroke', colors[link.source % 3])
                .attr('stroke-width', link.value / 5)
                .attr('opacity', 0.6);
        });
        
        // Labels
        ['Swaps', 'FX', 'Options'].forEach((label, i) => {
            svg.append('text')
                .attr('x', 10)
                .attr('y', 25 + i * 40)
                .attr('fill', 'var(--text-secondary)')
                .attr('font-size', '0.65rem')
                .text(label);
        });
        
        ['EPE', 'CVA'].forEach((label, i) => {
            svg.append('text')
                .attr('x', width - 30)
                .attr('y', 25 + i * 60)
                .attr('fill', 'var(--text-secondary)')
                .attr('font-size', '0.65rem')
                .text(label);
        });
    },
    
    initDistributionChart() {
        const ctx = document.getElementById('dist-chart')?.getContext('2d');
        if (!ctx) return;
        
        // Generate normal distribution data
        const data = [];
        for (let x = -4; x <= 4; x += 0.2) {
            data.push({
                x: x,
                y: Math.exp(-0.5 * x * x) / Math.sqrt(2 * Math.PI)
            });
        }
        
        new Chart(ctx, {
            type: 'line',
            data: {
                datasets: [{
                    data: data,
                    borderColor: 'rgba(99, 102, 241, 1)',
                    backgroundColor: 'rgba(99, 102, 241, 0.2)',
                    fill: true,
                    tension: 0.4,
                    pointRadius: 0
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: { legend: { display: false } },
                scales: {
                    x: { 
                        type: 'linear',
                        grid: { color: 'rgba(255,255,255,0.05)' },
                        ticks: { color: 'rgba(255,255,255,0.5)' }
                    },
                    y: { 
                        grid: { color: 'rgba(255,255,255,0.05)' },
                        ticks: { color: 'rgba(255,255,255,0.5)' }
                    }
                }
            }
        });
    }
};

// ============================================
// Real-time Data Effects
// ============================================

function initRealtimeEffects() {
    // Simulate real-time value updates
    setInterval(() => {
        const values = document.querySelectorAll('.stat-value, .metric-value');
        const randomIndex = Math.floor(Math.random() * values.length);
        const el = values[randomIndex];
        
        if (el) {
            el.classList.add('value-updated');
            setTimeout(() => el.classList.remove('value-updated'), 500);
        }
    }, 3000);
}

// ============================================
// Keyboard Shortcuts
// ============================================

function initKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        // Escape closes all panels/modals
        if (e.key === 'Escape') {
            document.querySelectorAll('.modal-overlay.active, .alert-panel.active, .theme-panel.active, .ai-panel.active').forEach(el => {
                el.classList.remove('active');
            });
        }
        
        // Ctrl+Shift+A - Toggle Alerts
        if (e.ctrlKey && e.shiftKey && e.key === 'A') {
            e.preventDefault();
            alertSystem.toggle();
        }
        
        // Ctrl+Shift+T - Toggle Theme
        if (e.ctrlKey && e.shiftKey && e.key === 'T') {
            e.preventDefault();
            themeCustomizer.toggle();
        }
        
        // Ctrl+Shift+I - Toggle AI
        if (e.ctrlKey && e.shiftKey && e.key === 'I') {
            e.preventDefault();
            aiAssistant.toggle();
        }
    });
}

// ============================================
// Advanced Visual Effects
// ============================================

// Ripple Effect
function initRippleEffect() {
    document.querySelectorAll('.ripple-container, .btn, .nav-item, .bento-card').forEach(el => {
        el.addEventListener('click', function(e) {
            const rect = this.getBoundingClientRect();
            const ripple = document.createElement('span');
            ripple.className = 'ripple';
            
            const size = Math.max(rect.width, rect.height) * 2;
            ripple.style.width = ripple.style.height = size + 'px';
            ripple.style.left = (e.clientX - rect.left - size / 2) + 'px';
            ripple.style.top = (e.clientY - rect.top - size / 2) + 'px';
            
            this.style.position = 'relative';
            this.style.overflow = 'hidden';
            this.appendChild(ripple);
            
            setTimeout(() => ripple.remove(), 600);
        });
    });
}

// Animated Counter (morphing numbers)
function animateValue(element, start, end, duration = 1000) {
    const startTime = performance.now();
    const isDecimal = String(end).includes('.') || Math.abs(end) < 100;
    const decimals = isDecimal ? 2 : 0;
    
    function update(currentTime) {
        const elapsed = currentTime - startTime;
        const progress = Math.min(elapsed / duration, 1);
        
        // Easing function (ease-out-cubic)
        const easeOut = 1 - Math.pow(1 - progress, 3);
        const current = start + (end - start) * easeOut;
        
        element.textContent = current.toFixed(decimals);
        element.classList.add('updating');
        
        if (progress < 1) {
            requestAnimationFrame(update);
        } else {
            element.classList.remove('updating');
        }
    }
    
    requestAnimationFrame(update);
}

// Sparkline Generator
function createSparkline(container, data, options = {}) {
    const width = options.width || 60;
    const height = options.height || 20;
    const color = options.color || 'var(--primary)';
    
    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    
    const points = data.map((val, i) => {
        const x = (i / (data.length - 1)) * width;
        const y = height - ((val - min) / range) * height;
        return `${x},${y}`;
    }).join(' ');
    
    const fillPoints = `0,${height} ${points} ${width},${height}`;
    
    container.innerHTML = `
        <svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
            <defs>
                <linearGradient id="sparkline-grad-${Date.now()}" x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" style="stop-color:${color};stop-opacity:0.3"/>
                    <stop offset="100%" style="stop-color:${color};stop-opacity:0"/>
                </linearGradient>
            </defs>
            <polygon points="${fillPoints}" fill="url(#sparkline-grad-${Date.now()})"/>
            <polyline points="${points}" fill="none" stroke="${color}" stroke-width="1.5"/>
        </svg>
    `;
}

// Progress Ring
function updateProgressRing(element, percentage) {
    const circle = element.querySelector('.progress-ring-progress');
    const valueEl = element.querySelector('.progress-ring-value');
    
    if (circle) {
        const circumference = 2 * Math.PI * 36; // radius = 36
        const offset = circumference - (percentage / 100) * circumference;
        circle.style.strokeDashoffset = offset;
    }
    
    if (valueEl) {
        valueEl.textContent = Math.round(percentage) + '%';
    }
}

// Gauge Needle Animation
function updateGauge(element, value, min = 0, max = 100) {
    const needle = element.querySelector('.gauge-needle');
    if (!needle) return;
    
    const percentage = (value - min) / (max - min);
    const angle = -90 + (percentage * 180); // -90 to 90 degrees
    needle.style.transform = `translateX(-50%) rotate(${angle}deg)`;
}

// Context Menu
const contextMenu = {
    menu: null,
    
    init() {
        // Create menu element
        this.menu = document.createElement('div');
        this.menu.className = 'context-menu';
        this.menu.innerHTML = `
            <div class="context-menu-item" data-action="view">
                <i class="fas fa-eye"></i>
                <span>View Details</span>
                <span class="context-menu-shortcut">Enter</span>
            </div>
            <div class="context-menu-item" data-action="edit">
                <i class="fas fa-edit"></i>
                <span>Edit</span>
                <span class="context-menu-shortcut">E</span>
            </div>
            <div class="context-menu-item" data-action="export">
                <i class="fas fa-download"></i>
                <span>Export</span>
            </div>
            <div class="context-menu-divider"></div>
            <div class="context-menu-item" data-action="analyze">
                <i class="fas fa-chart-line"></i>
                <span>Analyze</span>
            </div>
            <div class="context-menu-item" data-action="whatif">
                <i class="fas fa-flask"></i>
                <span>What-If</span>
            </div>
        `;
        document.body.appendChild(this.menu);
        
        // Handle clicks
        this.menu.querySelectorAll('.context-menu-item').forEach(item => {
            item.addEventListener('click', (e) => {
                const action = item.dataset.action;
                this.handleAction(action);
                this.hide();
            });
        });
        
        // Hide on click outside
        document.addEventListener('click', () => this.hide());
        
        // Show on right-click
        document.querySelectorAll('.bento-card, .glass-card, .trade-row').forEach(el => {
            el.addEventListener('contextmenu', (e) => {
                e.preventDefault();
                this.show(e.clientX, e.clientY, el);
            });
        });
    },
    
    show(x, y, target) {
        this.menu.style.left = x + 'px';
        this.menu.style.top = y + 'px';
        this.menu.classList.add('visible');
        this.currentTarget = target;
        
        // Adjust if off-screen
        const rect = this.menu.getBoundingClientRect();
        if (rect.right > window.innerWidth) {
            this.menu.style.left = (x - rect.width) + 'px';
        }
        if (rect.bottom > window.innerHeight) {
            this.menu.style.top = (y - rect.height) + 'px';
        }
    },
    
    hide() {
        this.menu.classList.remove('visible');
    },
    
    handleAction(action) {
        switch(action) {
            case 'view':
                showToast('info', 'View Details', 'Opening detail view...');
                break;
            case 'edit':
                showToast('info', 'Edit Mode', 'Entering edit mode...');
                break;
            case 'export':
                showToast('success', 'Export', 'Data exported to clipboard');
                break;
            case 'analyze':
                showToast('info', 'Analysis', 'Running analysis...');
                break;
            case 'whatif':
                whatIfSimulator.open();
                break;
        }
    }
};

// Rich Tooltips
const richTooltip = {
    tooltip: null,
    
    init() {
        this.tooltip = document.createElement('div');
        this.tooltip.className = 'rich-tooltip';
        this.tooltip.innerHTML = `
            <div class="rich-tooltip-arrow"></div>
            <div class="rich-tooltip-title"></div>
            <div class="rich-tooltip-content"></div>
        `;
        document.body.appendChild(this.tooltip);
        
        // Add tooltips to elements with data-tooltip
        document.querySelectorAll('[data-tooltip]').forEach(el => {
            el.addEventListener('mouseenter', (e) => this.show(e.target));
            el.addEventListener('mouseleave', () => this.hide());
        });
    },
    
    show(element) {
        const data = element.dataset;
        const title = data.tooltipTitle || '';
        const content = data.tooltip || '';
        
        this.tooltip.querySelector('.rich-tooltip-title').textContent = title;
        this.tooltip.querySelector('.rich-tooltip-content').textContent = content;
        
        const rect = element.getBoundingClientRect();
        this.tooltip.style.left = (rect.left + rect.width / 2 - this.tooltip.offsetWidth / 2) + 'px';
        this.tooltip.style.top = (rect.bottom + 10) + 'px';
        
        this.tooltip.classList.add('visible');
    },
    
    hide() {
        this.tooltip.classList.remove('visible');
    }
};

// Skeleton Loading
function showSkeleton(container, type = 'card') {
    const templates = {
        card: `
            <div class="skeleton skeleton-card">
                <div class="skeleton skeleton-text short"></div>
                <div class="skeleton skeleton-text long" style="height:2em;margin:12px 0;"></div>
                <div class="skeleton skeleton-text medium"></div>
            </div>
        `,
        table: `
            <div class="skeleton" style="padding:12px;">
                ${Array(5).fill('<div class="skeleton skeleton-text long" style="margin-bottom:12px;"></div>').join('')}
            </div>
        `,
        chart: `
            <div class="skeleton" style="height:200px;border-radius:var(--radius-md);"></div>
        `
    };
    
    container.innerHTML = templates[type] || templates.card;
}

function hideSkeleton(container) {
    container.querySelectorAll('.skeleton').forEach(el => el.remove());
}

// Shine Effect on Cards
function initShineEffect() {
    document.querySelectorAll('.bento-card, .glass-card').forEach(card => {
        card.classList.add('shine-effect');
    });
}

// Stagger Animation on View Change
function applyStaggerAnimation(container) {
    container.classList.add('stagger-container');
    // Force reflow to restart animation
    container.offsetHeight;
}

// Scroll-based Header Blur
function initScrollBlur() {
    const header = document.querySelector('.top-bar');
    if (!header) return;
    
    header.classList.add('blur-on-scroll');
    
    window.addEventListener('scroll', () => {
        if (window.scrollY > 20) {
            header.classList.add('scrolled');
        } else {
            header.classList.remove('scrolled');
        }
    }, { passive: true });
}

// Aurora Background
function initAuroraBackground() {
    const mainContent = document.querySelector('.main-content');
    if (mainContent) {
        mainContent.classList.add('aurora-bg', 'mesh-gradient-bg');
    }
}

// Enhanced Value Updates with Animation
function updateValueWithAnimation(selector, newValue, format = 'number') {
    const element = document.querySelector(selector);
    if (!element) return;
    
    const oldValue = parseFloat(element.textContent.replace(/[^0-9.-]/g, '')) || 0;
    
    if (format === 'currency') {
        animateValue(element, oldValue, newValue, 800);
        setTimeout(() => {
            element.textContent = '$' + newValue.toFixed(1) + 'M';
        }, 850);
    } else if (format === 'percent') {
        animateValue(element, oldValue, newValue, 800);
        setTimeout(() => {
            element.textContent = newValue.toFixed(1) + '%';
        }, 850);
    } else {
        animateValue(element, oldValue, newValue, 800);
    }
    
    element.classList.add('morph-value', 'counter-animate');
    setTimeout(() => element.classList.remove('counter-animate'), 500);
}

// Initialize all visual effects
function initVisualEffects() {
    initRippleEffect();
    initShineEffect();
    initScrollBlur();
    initAuroraBackground();
    contextMenu.init();
    richTooltip.init();
    
    // Add gradient border effect to key cards
    document.querySelectorAll('.summary-card, .metric-card').forEach(card => {
        card.classList.add('gradient-border', 'inner-glow');
    });
    
    // Add stagger animation to card grids
    document.querySelectorAll('.bento-grid, .metrics-grid').forEach(grid => {
        applyStaggerAnimation(grid);
    });
    
    // Demo sparklines
    document.querySelectorAll('.sparkline-container').forEach(container => {
        const randomData = Array.from({length: 10}, () => Math.random() * 100);
        createSparkline(container, randomData);
    });
}

// Initialization
// ============================================

async function init() {
    try {
        // Initialize systems
        new ParticleSystem(document.getElementById('particle-canvas'));
        new CommandPalette();
        
        // Initialize advanced features (with error handling for each)
        try { alertSystem.init(); } catch(e) { console.error('alertSystem init error:', e); }
        try { themeCustomizer.init(); } catch(e) { console.error('themeCustomizer init error:', e); }
        try { whatIfSimulator.init(); } catch(e) { console.error('whatIfSimulator init error:', e); }
        try { reportGenerator.init(); } catch(e) { console.error('reportGenerator init error:', e); }
        try { aiAssistant.init(); } catch(e) { console.error('aiAssistant init error:', e); }
        try { analytics3D.init(); } catch(e) { console.error('analytics3D init error:', e); }
        try { initRealtimeEffects(); } catch(e) { console.error('initRealtimeEffects error:', e); }
        try { initKeyboardShortcuts(); } catch(e) { console.error('initKeyboardShortcuts error:', e); }
        try { initVisualEffects(); } catch(e) { console.error('initVisualEffects error:', e); }
        
        // Initialize UI
        initTheme();
        initNavigation();
        initPortfolioControls();
        initScenarioControls();
        try { initEnhancedScenarioControls(); } catch(e) { console.error('initEnhancedScenarioControls error:', e); }
        initQuickActions();
        initChartControls();
        initTiltEffect();
        
        // Initialize enhanced views
        try { initRiskView(); } catch(e) { console.error('initRiskView error:', e); }
        try { initExposureView(); } catch(e) { console.error('initExposureView error:', e); }
        try { initImpactChart(); } catch(e) { console.error('initImpactChart error:', e); }
        try { initGraphTab(); } catch(e) { console.error('initGraphTab error:', e); }
        
        // Load data
        showLoading('Loading dashboard...');
        
        try {
            await Promise.all([fetchPortfolio(), fetchRiskMetrics(), fetchExposure()]);
        } catch (e) {
            console.error('Initial load failed:', e);
        }
        
    } catch (e) {
        console.error('Init error:', e);
    } finally {
        // Always hide loading
        hideLoading();
    }
    
    // Connect WebSocket
    try { connectWebSocket(); } catch(e) { console.error('WebSocket error:', e); }
    
    // Periodic refresh
    setInterval(() => {
        fetchPortfolio();
        fetchRiskMetrics();
    }, REFRESH_INTERVAL);
    
    // Override run scenario button
    const runBtn = document.getElementById('run-scenario');
    if (runBtn) {
        runBtn.removeEventListener('click', runScenario);
        runBtn.addEventListener('click', runEnhancedScenario);
    }
}

document.addEventListener('DOMContentLoaded', init);

// ============================================
// Task 5.1: GraphManager Class
// ============================================

/**
 * GraphManager handles graph data fetching, state management,
 * and WebSocket update processing for computation graph visualisation.
 */
class GraphManager {
    constructor() {
        this.graphs = {};           // trade_id -> ComputationGraph
        this.subscriptions = new Set();  // subscribed trade IDs
        this.currentTradeId = null;
        this.listeners = new Map(); // event listeners
    }

    /**
     * Fetch computation graph from REST API
     * @param {string|null} tradeId - Trade ID to fetch, or null for all trades
     * @returns {Promise<object>} Graph data with nodes, links, and metadata
     */
    async fetchGraph(tradeId = null) {
        const url = tradeId
            ? `${API_BASE}/graph?trade_id=${tradeId}`
            : `${API_BASE}/graph`;

        const response = await fetch(url);
        if (!response.ok) {
            const error = await response.json().catch(() => ({ message: 'Unknown error' }));
            throw new Error(error.message || `HTTP ${response.status}`);
        }

        const data = await response.json();
        this.graphs[tradeId || 'all'] = data;
        this.currentTradeId = tradeId;

        // Notify listeners
        this.notifyListeners('graph_loaded', { tradeId, data });

        return data;
    }

    /**
     * Handle WebSocket graph_update message
     * @param {object} message - WebSocket message with type and data
     */
    handleGraphUpdate(message) {
        if (message.type !== 'graph_update') return;

        const { trade_id, updated_nodes } = message.data;

        // Only process if subscribed
        if (!this.subscriptions.has(trade_id)) return;

        // Apply differential update
        const graph = this.graphs[trade_id];
        if (graph) {
            updated_nodes.forEach(update => {
                const node = graph.nodes.find(n => n.id === update.id);
                if (node) {
                    node.value = update.value;
                }
            });
        }

        // Notify listeners
        this.notifyListeners('graph_update', { tradeId: trade_id, updatedNodes: updated_nodes });
    }

    /**
     * Subscribe to graph updates for a specific trade
     * @param {string} tradeId - Trade ID to subscribe to
     */
    subscribe(tradeId) {
        this.subscriptions.add(tradeId);
    }

    /**
     * Unsubscribe from graph updates for a specific trade
     * @param {string} tradeId - Trade ID to unsubscribe from
     */
    unsubscribe(tradeId) {
        this.subscriptions.delete(tradeId);
    }

    /**
     * Check if a trade is subscribed
     * @param {string} tradeId - Trade ID to check
     * @returns {boolean} True if subscribed
     */
    isSubscribed(tradeId) {
        return this.subscriptions.has(tradeId);
    }

    /**
     * Add event listener
     * @param {string} event - Event name
     * @param {function} callback - Callback function
     */
    addListener(event, callback) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, []);
        }
        this.listeners.get(event).push(callback);
    }

    /**
     * Remove event listener
     * @param {string} event - Event name
     * @param {function} callback - Callback function to remove
     */
    removeListener(event, callback) {
        if (this.listeners.has(event)) {
            const callbacks = this.listeners.get(event);
            const index = callbacks.indexOf(callback);
            if (index > -1) {
                callbacks.splice(index, 1);
            }
        }
    }

    /**
     * Notify all listeners for an event
     * @param {string} event - Event name
     * @param {object} data - Event data
     */
    notifyListeners(event, data) {
        if (this.listeners.has(event)) {
            this.listeners.get(event).forEach(callback => {
                try {
                    callback(data);
                } catch (e) {
                    console.error(`GraphManager listener error (${event}):`, e);
                }
            });
        }
    }

    /**
     * Get current graph data
     * @param {string|null} tradeId - Trade ID or null for all
     * @returns {object|null} Graph data or null
     */
    getGraph(tradeId = null) {
        return this.graphs[tradeId || 'all'] || null;
    }

    /**
     * Clear all cached graphs
     */
    clearCache() {
        this.graphs = {};
    }
}

// Global GraphManager instance
const graphManager = new GraphManager();

// ============================================
// Task 5.1: Graph State
// ============================================

/**
 * Graph visualisation state
 */
const graphState = {
    nodes: [],           // GraphNode array
    links: [],           // GraphEdge array
    metadata: {},        // GraphMetadata
    simulation: null,    // D3 force simulation
    svg: null,           // SVG element
    g: null,             // Main group (for zoom transform)
    zoom: null,          // D3 zoom behavior
    selectedNode: null,  // Currently selected node
    searchQuery: '',     // Search query
    highlightPath: [],   // Highlighted path nodes
    lodEnabled: false,   // Level of Detail enabled
    renderMode: 'svg',   // 'svg' | 'canvas'
};

/**
 * Node type colour mapping
 * - input: blue (#3b82f6)
 * - intermediate: grey (#6b7280)
 * - output: green (#22c55e)
 * - sensitivity: orange (#f97316)
 */
const nodeColors = {
    input: '#3b82f6',
    intermediate: '#6b7280',
    output: '#22c55e',
    sensitivity: '#f97316',
};

/**
 * Get colour for a node based on its group
 * @param {object} node - Graph node
 * @returns {string} Colour hex code
 */
function getNodeColor(node) {
    if (node.is_sensitivity_target) return nodeColors.sensitivity;
    return nodeColors[node.group] || nodeColors.intermediate;
}

// ============================================
// Task 5.2: D3.js Graph Rendering
// ============================================

/**
 * Initialise the graph view with SVG and D3 force simulation
 */
function initGraphView() {
    const container = document.getElementById('graph-container');
    if (!container) return;

    // Clear any existing content
    container.innerHTML = '';

    // Get container dimensions
    const width = container.clientWidth || 800;
    const height = container.clientHeight || 600;

    // Create SVG element
    graphState.svg = d3.select(container)
        .append('svg')
        .attr('width', '100%')
        .attr('height', '100%')
        .attr('viewBox', `0 0 ${width} ${height}`)
        .attr('class', 'graph-svg');

    // Create main group for zoom/pan transforms
    graphState.g = graphState.svg.append('g')
        .attr('class', 'graph-main-group');

    // Add arrow marker for directed edges
    graphState.svg.append('defs').append('marker')
        .attr('id', 'arrowhead')
        .attr('viewBox', '-0 -5 10 10')
        .attr('refX', 20)
        .attr('refY', 0)
        .attr('orient', 'auto')
        .attr('markerWidth', 6)
        .attr('markerHeight', 6)
        .append('path')
        .attr('d', 'M0,-5L10,0L0,5')
        .attr('fill', '#64748b');

    // Initialise force simulation
    graphState.simulation = d3.forceSimulation()
        .force('link', d3.forceLink()
            .id(d => d.id)
            .distance(80))
        .force('charge', d3.forceManyBody()
            .strength(-300))
        .force('center', d3.forceCenter(width / 2, height / 2))
        .force('collision', d3.forceCollide().radius(30));

    // Task 5.3: Setup zoom behaviour
    setupZoomBehavior();

    // Listen for graph loaded events
    graphManager.addListener('graph_loaded', ({ data }) => {
        renderGraph(data);
    });

    // Listen for graph update events
    graphManager.addListener('graph_update', ({ updatedNodes }) => {
        updateGraphNodes(updatedNodes);
    });
}

/**
 * Render the computation graph with D3.js force-directed layout
 * @param {object} data - Graph data with nodes, links, and metadata
 */
function renderGraph(data) {
    if (!graphState.g || !graphState.simulation) {
        console.warn('Graph view not initialised');
        return;
    }

    // Store state
    graphState.nodes = data.nodes || [];
    graphState.links = data.links || [];
    graphState.metadata = data.metadata || {};

    // Clear existing elements
    graphState.g.selectAll('.link').remove();
    graphState.g.selectAll('.node').remove();
    graphState.g.selectAll('.node-label').remove();

    // Create links (edges)
    const links = graphState.g.append('g')
        .attr('class', 'links')
        .selectAll('line')
        .data(graphState.links)
        .enter()
        .append('line')
        .attr('class', 'link')
        .attr('stroke', '#64748b')
        .attr('stroke-opacity', 0.6)
        .attr('stroke-width', 1.5)
        .attr('marker-end', 'url(#arrowhead)');

    // Create node groups
    const nodeGroups = graphState.g.append('g')
        .attr('class', 'nodes')
        .selectAll('g')
        .data(graphState.nodes)
        .enter()
        .append('g')
        .attr('class', 'node-group');

    // Add circles for nodes
    const nodes = nodeGroups.append('circle')
        .attr('class', 'node')
        .attr('r', d => d.is_sensitivity_target ? 12 : 8)
        .attr('fill', d => getNodeColor(d))
        .attr('stroke', '#fff')
        .attr('stroke-width', 2)
        .style('cursor', 'pointer');

    // Add labels for nodes
    const labels = nodeGroups.append('text')
        .attr('class', 'node-label')
        .attr('dx', 15)
        .attr('dy', 4)
        .attr('font-size', '10px')
        .attr('fill', 'var(--text-secondary, #94a3b8)')
        .text(d => d.label);

    // Task 5.3: Setup node drag behaviour
    nodeGroups.call(d3.drag()
        .on('start', dragStarted)
        .on('drag', dragged)
        .on('end', dragEnded));

    // Add hover tooltip
    nodeGroups
        .on('mouseover', (event, d) => {
            showNodeTooltip(event, d);
        })
        .on('mouseout', () => {
            hideNodeTooltip();
        })
        .on('click', (event, d) => {
            selectNode(d);
        });

    // Update simulation
    graphState.simulation
        .nodes(graphState.nodes)
        .on('tick', () => {
            links
                .attr('x1', d => d.source.x)
                .attr('y1', d => d.source.y)
                .attr('x2', d => d.target.x)
                .attr('y2', d => d.target.y);

            nodeGroups
                .attr('transform', d => `translate(${d.x},${d.y})`);
        });

    graphState.simulation.force('link')
        .links(graphState.links);

    // Restart simulation
    graphState.simulation.alpha(1).restart();

    // Update stats panel (using extended version for Task 6.3, 7.2)
    updateGraphStatsPanelExtended();
}

/**
 * Update specific nodes after WebSocket update
 * @param {Array} updatedNodes - Array of node updates with id and value
 */
function updateGraphNodes(updatedNodes) {
    if (!graphState.g) return;

    updatedNodes.forEach(update => {
        // Update state
        const node = graphState.nodes.find(n => n.id === update.id);
        if (node) {
            node.value = update.value;
        }

        // Flash animation for updated nodes
        graphState.g.selectAll('.node-group')
            .filter(d => d.id === update.id)
            .select('circle')
            .transition()
            .duration(200)
            .attr('stroke', '#f97316')
            .attr('stroke-width', 4)
            .transition()
            .duration(300)
            .attr('stroke', '#fff')
            .attr('stroke-width', 2);
    });
}

// ============================================
// Task 5.3: Zoom, Pan, and Drag
// ============================================

/**
 * Setup D3.js zoom behaviour for pan and zoom
 */
function setupZoomBehavior() {
    if (!graphState.svg || !graphState.g) return;

    graphState.zoom = d3.zoom()
        .scaleExtent([0.1, 4])  // Min 10%, Max 400% zoom
        .on('zoom', (event) => {
            graphState.g.attr('transform', event.transform);

            // Adjust label visibility based on zoom level
            adjustLabelsForZoom(event.transform.k);
        });

    graphState.svg.call(graphState.zoom);
}

/**
 * Adjust label visibility based on zoom level
 * @param {number} scale - Current zoom scale
 */
function adjustLabelsForZoom(scale) {
    if (!graphState.g) return;

    // Hide labels when zoomed out, show when zoomed in
    const opacity = scale < 0.5 ? 0 : scale < 1 ? (scale - 0.5) * 2 : 1;

    graphState.g.selectAll('.node-label')
        .attr('opacity', opacity);
}

/**
 * Drag started handler
 */
function dragStarted(event, d) {
    if (!event.active) {
        graphState.simulation.alphaTarget(0.3).restart();
    }
    d.fx = d.x;
    d.fy = d.y;
}

/**
 * Dragging handler
 */
function dragged(event, d) {
    d.fx = event.x;
    d.fy = event.y;
}

/**
 * Drag ended handler
 */
function dragEnded(event, d) {
    if (!event.active) {
        graphState.simulation.alphaTarget(0);
    }
    // Optionally release the fixed position
    // d.fx = null;
    // d.fy = null;
}

/**
 * Reset zoom to default view
 */
function resetGraphZoom() {
    if (!graphState.svg || !graphState.zoom) return;

    graphState.svg.transition()
        .duration(500)
        .call(graphState.zoom.transform, d3.zoomIdentity);
}

/**
 * Zoom to fit all nodes
 */
function zoomToFit() {
    if (!graphState.svg || !graphState.zoom || !graphState.nodes.length) return;

    const bounds = graphState.g.node().getBBox();
    const parent = graphState.svg.node().getBoundingClientRect();
    const width = parent.width || 800;
    const height = parent.height || 600;

    const scale = Math.min(
        0.9 * width / bounds.width,
        0.9 * height / bounds.height,
        2  // Max scale
    );

    const translateX = (width - scale * bounds.width) / 2 - scale * bounds.x;
    const translateY = (height - scale * bounds.height) / 2 - scale * bounds.y;

    graphState.svg.transition()
        .duration(500)
        .call(
            graphState.zoom.transform,
            d3.zoomIdentity.translate(translateX, translateY).scale(scale)
        );
}

// ============================================
// Task 5.1: Graph Tab Navigation
// ============================================

/**
 * Navigate to graph view and optionally load a specific trade
 * @param {string|null} tradeId - Trade ID to load, or null for all
 */
async function navigateToGraph(tradeId = null) {
    navigateTo('graph');

    // Initialise graph view if needed
    if (!graphState.svg) {
        initGraphView();
    }

    // Show loading state
    const graphContent = document.getElementById('graph-content');
    const graphLoading = document.getElementById('graph-loading');
    if (graphContent) graphContent.style.display = 'none';
    if (graphLoading) graphLoading.style.display = 'flex';

    try {
        await graphManager.fetchGraph(tradeId);
        if (tradeId) {
            graphManager.subscribe(tradeId);
        }
    } catch (error) {
        console.error('Failed to load graph:', error);
        showToast('Failed to load computation graph', 'error');
    } finally {
        if (graphContent) graphContent.style.display = 'block';
        if (graphLoading) graphLoading.style.display = 'none';
    }
}

// ============================================
// Graph UI Helpers
// ============================================

/**
 * Show tooltip for a node
 * @param {Event} event - Mouse event
 * @param {object} node - Node data
 */
function showNodeTooltip(event, node) {
    let tooltip = document.getElementById('graph-tooltip');
    if (!tooltip) {
        tooltip = document.createElement('div');
        tooltip.id = 'graph-tooltip';
        tooltip.className = 'graph-tooltip glass';
        document.body.appendChild(tooltip);
    }

    const valueStr = node.value !== null && node.value !== undefined
        ? node.value.toFixed(4)
        : 'N/A';

    tooltip.innerHTML = `
        <div class="tooltip-header">
            <span class="tooltip-label">${node.label}</span>
            <span class="tooltip-type">${node.type}</span>
        </div>
        <div class="tooltip-body">
            <div class="tooltip-row">
                <span>Value:</span>
                <span>${valueStr}</span>
            </div>
            <div class="tooltip-row">
                <span>Group:</span>
                <span>${node.group}</span>
            </div>
            ${node.is_sensitivity_target ? '<div class="tooltip-badge">Sensitivity Target</div>' : ''}
        </div>
    `;

    tooltip.style.left = `${event.pageX + 15}px`;
    tooltip.style.top = `${event.pageY - 10}px`;
    tooltip.style.display = 'block';
}

/**
 * Hide node tooltip
 */
function hideNodeTooltip() {
    const tooltip = document.getElementById('graph-tooltip');
    if (tooltip) {
        tooltip.style.display = 'none';
    }
}

/**
 * Select a node and highlight its connections
 * @param {object} node - Node to select
 */
function selectNode(node) {
    graphState.selectedNode = node;

    // Reset all nodes/links to default opacity
    graphState.g.selectAll('.node').attr('opacity', 0.3);
    graphState.g.selectAll('.link').attr('opacity', 0.1);

    // Highlight selected node
    graphState.g.selectAll('.node-group')
        .filter(d => d.id === node.id)
        .select('.node')
        .attr('opacity', 1);

    // Highlight connected nodes and links
    const connectedIds = new Set([node.id]);
    graphState.links.forEach(link => {
        const sourceId = link.source.id || link.source;
        const targetId = link.target.id || link.target;

        if (sourceId === node.id || targetId === node.id) {
            connectedIds.add(sourceId);
            connectedIds.add(targetId);
        }
    });

    graphState.g.selectAll('.node-group')
        .filter(d => connectedIds.has(d.id))
        .select('.node')
        .attr('opacity', 1);

    graphState.g.selectAll('.link')
        .filter(d => {
            const sourceId = d.source.id || d.source;
            const targetId = d.target.id || d.target;
            return sourceId === node.id || targetId === node.id;
        })
        .attr('opacity', 0.8);

    // Update info panel
    updateNodeInfoPanel(node);
}

/**
 * Clear node selection
 */
function clearNodeSelection() {
    graphState.selectedNode = null;

    graphState.g.selectAll('.node').attr('opacity', 1);
    graphState.g.selectAll('.link').attr('opacity', 0.6);

    updateNodeInfoPanel(null);
}

/**
 * Update node info panel
 * @param {object|null} node - Selected node or null
 */
function updateNodeInfoPanel(node) {
    const panel = document.getElementById('node-info-panel');
    if (!panel) return;

    if (!node) {
        panel.innerHTML = '<div class="no-selection">Click a node to see details</div>';
        return;
    }

    const valueStr = node.value !== null && node.value !== undefined
        ? node.value.toFixed(6)
        : 'N/A';

    panel.innerHTML = `
        <div class="node-info-header">
            <span class="node-info-id">${node.id}</span>
            <span class="node-info-type" style="background: ${getNodeColor(node)}">${node.type}</span>
        </div>
        <div class="node-info-body">
            <div class="info-row">
                <span class="info-label">Label</span>
                <span class="info-value">${node.label}</span>
            </div>
            <div class="info-row">
                <span class="info-label">Value</span>
                <span class="info-value">${valueStr}</span>
            </div>
            <div class="info-row">
                <span class="info-label">Group</span>
                <span class="info-value">${node.group}</span>
            </div>
            ${node.is_sensitivity_target ? '<div class="sensitivity-badge"><i class="fas fa-bullseye"></i> Sensitivity Target</div>' : ''}
        </div>
    `;
}

/**
 * Update graph statistics panel
 */
function updateGraphStatsPanel() {
    const nodeCountEl = document.getElementById('graph-node-count');
    const edgeCountEl = document.getElementById('graph-edge-count');
    const depthEl = document.getElementById('graph-depth');
    const generatedAtEl = document.getElementById('graph-generated-at');

    if (nodeCountEl) nodeCountEl.textContent = graphState.metadata.node_count || 0;
    if (edgeCountEl) edgeCountEl.textContent = graphState.metadata.edge_count || 0;
    if (depthEl) depthEl.textContent = graphState.metadata.depth || 0;
    if (generatedAtEl) {
        const date = graphState.metadata.generated_at
            ? new Date(graphState.metadata.generated_at).toLocaleString()
            : 'N/A';
        generatedAtEl.textContent = date;
    }
}

/**
 * Initialise graph view controls
 */
function initGraphControls() {
    // Trade selector
    const tradeSelector = document.getElementById('graph-trade-selector');
    if (tradeSelector) {
        tradeSelector.addEventListener('change', async (e) => {
            const tradeId = e.target.value || null;
            try {
                await graphManager.fetchGraph(tradeId);
            } catch (error) {
                console.error('Failed to load graph:', error);
                showToast('Failed to load graph', 'error');
            }
        });
    }

    // Zoom controls
    document.getElementById('graph-zoom-in')?.addEventListener('click', () => {
        if (graphState.svg && graphState.zoom) {
            graphState.svg.transition()
                .duration(300)
                .call(graphState.zoom.scaleBy, 1.3);
        }
    });

    document.getElementById('graph-zoom-out')?.addEventListener('click', () => {
        if (graphState.svg && graphState.zoom) {
            graphState.svg.transition()
                .duration(300)
                .call(graphState.zoom.scaleBy, 0.7);
        }
    });

    document.getElementById('graph-zoom-reset')?.addEventListener('click', resetGraphZoom);
    document.getElementById('graph-zoom-fit')?.addEventListener('click', zoomToFit);

    // Clear selection
    document.getElementById('graph-clear-selection')?.addEventListener('click', clearNodeSelection);

    // Task 6.2: Initialise search controls
    initSearchControls();

    // Task 6.3: Initialise sensitivity path controls
    initSensitivityPathControls();
}

// ============================================
// Graph View Initialisation (Task 5.1)
// ============================================

/**
 * Initialise the graph view tab
 * Called from main init() function
 */
function initGraphTab() {
    initGraphView();
    initGraphControls();

    // Integrate with WebSocket handler for graph_update messages
    // This is handled in handleWsMessage but we add the GraphManager callback
}

// ============================================
// Task 6.3: Sensitivity Path Highlight
// ============================================

/**
 * Sensitivity path state
 */
const sensitivityPathState = {
    paths: [],              // All computed sensitivity paths
    highlightedPath: null,  // Currently highlighted path
    isEnabled: false,       // Whether sensitivity path highlighting is enabled
};

/**
 * Find all sensitivity target nodes
 * @param {Array} nodes - Array of graph nodes
 * @returns {Array} Array of sensitivity target node IDs
 */
function findSensitivityTargets(nodes) {
    return nodes
        .filter(n => n.is_sensitivity_target)
        .map(n => n.id);
}

/**
 * Find all output nodes
 * @param {Array} nodes - Array of graph nodes
 * @returns {Array} Array of output node IDs
 */
function findOutputNodes(nodes) {
    return nodes
        .filter(n => n.group === 'output' || n.type === 'output')
        .map(n => n.id);
}

/**
 * Build adjacency list from links
 * @param {Array} links - Array of graph links/edges
 * @returns {Object} Adjacency list { nodeId: [connectedNodeIds] }
 */
function buildAdjacencyList(links) {
    const adjacency = {};
    links.forEach(link => {
        const source = link.source.id || link.source;
        const target = link.target.id || link.target;
        if (!adjacency[source]) adjacency[source] = [];
        adjacency[source].push(target);
    });
    return adjacency;
}

/**
 * Find path from source to target using BFS
 * @param {string} source - Source node ID
 * @param {string} target - Target node ID
 * @param {Object} adjacency - Adjacency list
 * @returns {Array|null} Array of node IDs in path, or null if no path
 */
function findPathBFS(source, target, adjacency) {
    if (source === target) return [source];

    const visited = new Set();
    const queue = [[source, [source]]];

    while (queue.length > 0) {
        const [current, path] = queue.shift();

        if (visited.has(current)) continue;
        visited.add(current);

        const neighbours = adjacency[current] || [];
        for (const neighbour of neighbours) {
            const newPath = [...path, neighbour];
            if (neighbour === target) {
                return newPath;
            }
            if (!visited.has(neighbour)) {
                queue.push([neighbour, newPath]);
            }
        }
    }

    return null;
}

/**
 * Find all sensitivity paths (from sensitivity targets to outputs)
 * @param {Array} nodes - Array of graph nodes
 * @param {Array} links - Array of graph links
 * @returns {Array} Array of path objects { from, to, path }
 */
function findAllSensitivityPaths(nodes, links) {
    const sensitivityTargets = findSensitivityTargets(nodes);
    const outputs = findOutputNodes(nodes);
    const adjacency = buildAdjacencyList(links);

    const paths = [];
    for (const source of sensitivityTargets) {
        for (const target of outputs) {
            const path = findPathBFS(source, target, adjacency);
            if (path) {
                paths.push({ from: source, to: target, path });
            }
        }
    }

    return paths;
}

/**
 * Get edges that are part of a path
 * @param {Array} path - Array of node IDs in path
 * @param {Array} links - Array of graph links
 * @returns {Array} Array of link objects that form the path
 */
function getEdgesForPath(path, links) {
    const edges = [];
    for (let i = 0; i < path.length - 1; i++) {
        const source = path[i];
        const target = path[i + 1];
        const edge = links.find(l => {
            const lSource = l.source.id || l.source;
            const lTarget = l.target.id || l.target;
            return lSource === source && lTarget === target;
        });
        if (edge) edges.push(edge);
    }
    return edges;
}

/**
 * Highlight a sensitivity path on the graph
 * @param {Object} pathObj - Path object { from, to, path }
 */
function highlightSensitivityPath(pathObj) {
    if (!graphState.g || !pathObj) return;

    const pathNodeIds = new Set(pathObj.path);

    // Dim all nodes and links
    graphState.g.selectAll('.node').attr('opacity', 0.2);
    graphState.g.selectAll('.link').attr('opacity', 0.1).attr('stroke', '#64748b');

    // Highlight path nodes
    graphState.g.selectAll('.node-group')
        .filter(d => pathNodeIds.has(d.id))
        .select('.node')
        .attr('opacity', 1)
        .attr('stroke', '#f97316')
        .attr('stroke-width', 3);

    // Highlight path edges
    const pathEdges = getEdgesForPath(pathObj.path, graphState.links);
    pathEdges.forEach(edge => {
        graphState.g.selectAll('.link')
            .filter(d => {
                const dSource = d.source.id || d.source;
                const dTarget = d.target.id || d.target;
                const eSource = edge.source.id || edge.source;
                const eTarget = edge.target.id || edge.target;
                return dSource === eSource && dTarget === eTarget;
            })
            .attr('opacity', 1)
            .attr('stroke', '#f97316')
            .attr('stroke-width', 3);
    });

    sensitivityPathState.highlightedPath = pathObj;
}

/**
 * Clear sensitivity path highlighting
 */
function clearSensitivityPathHighlight() {
    if (!graphState.g) return;

    // Restore all nodes
    graphState.g.selectAll('.node')
        .attr('opacity', 1)
        .attr('stroke', '#fff')
        .attr('stroke-width', 2);

    // Restore all links
    graphState.g.selectAll('.link')
        .attr('opacity', 0.6)
        .attr('stroke', '#64748b')
        .attr('stroke-width', 1.5);

    sensitivityPathState.highlightedPath = null;
}

/**
 * Toggle sensitivity path highlighting
 */
function toggleSensitivityPathHighlight() {
    sensitivityPathState.isEnabled = !sensitivityPathState.isEnabled;

    if (sensitivityPathState.isEnabled) {
        // Compute paths if not already done
        if (sensitivityPathState.paths.length === 0 && graphState.nodes.length > 0) {
            sensitivityPathState.paths = findAllSensitivityPaths(graphState.nodes, graphState.links);
        }

        // Highlight first path if available
        if (sensitivityPathState.paths.length > 0) {
            highlightSensitivityPath(sensitivityPathState.paths[0]);
        }

        // Update UI
        updateSensitivityPathSelector();
    } else {
        clearSensitivityPathHighlight();
    }

    // Update button state
    const btn = document.getElementById('sensitivity-path-toggle');
    if (btn) {
        btn.classList.toggle('active', sensitivityPathState.isEnabled);
    }
}

/**
 * Update sensitivity path selector dropdown
 */
function updateSensitivityPathSelector() {
    const selector = document.getElementById('sensitivity-path-selector');
    if (!selector) return;

    selector.innerHTML = '';

    sensitivityPathState.paths.forEach((pathObj, index) => {
        const option = document.createElement('option');
        option.value = index;
        option.textContent = `${pathObj.from} → ${pathObj.to} (${pathObj.path.length} nodes)`;
        selector.appendChild(option);
    });

    selector.style.display = sensitivityPathState.paths.length > 0 ? 'block' : 'none';
}

/**
 * Select a specific sensitivity path
 * @param {number} index - Index of path to select
 */
function selectSensitivityPath(index) {
    if (index >= 0 && index < sensitivityPathState.paths.length) {
        highlightSensitivityPath(sensitivityPathState.paths[index]);
    }
}

/**
 * Initialise sensitivity path controls
 */
function initSensitivityPathControls() {
    // Toggle button
    const toggleBtn = document.getElementById('sensitivity-path-toggle');
    if (toggleBtn) {
        toggleBtn.addEventListener('click', toggleSensitivityPathHighlight);
    }

    // Path selector
    const selector = document.getElementById('sensitivity-path-selector');
    if (selector) {
        selector.addEventListener('change', (e) => {
            selectSensitivityPath(parseInt(e.target.value, 10));
        });
    }
}

// ============================================
// Task 7.2: Node Type Statistics Chart
// ============================================

/**
 * Node type chart state
 */
const nodeTypeChartState = {
    chartInstance: null,
    typeCounts: {},
};

/**
 * Colour palette for node types
 */
const nodeTypeColors = {
    input: '#3b82f6',     // Blue
    output: '#22c55e',    // Green
    add: '#f59e0b',       // Amber
    mul: '#8b5cf6',       // Violet
    exp: '#ec4899',       // Pink
    log: '#14b8a6',       // Teal
    sqrt: '#06b6d4',      // Cyan
    div: '#f97316',       // Orange
    default: '#6b7280',   // Grey
};

/**
 * Count nodes by type
 * @param {Array} nodes - Array of graph nodes
 * @returns {Object} Object with type keys and count values
 */
function countNodesByType(nodes) {
    const counts = {};
    nodes.forEach(node => {
        const type = node.type || 'unknown';
        counts[type] = (counts[type] || 0) + 1;
    });
    return counts;
}

/**
 * Sort type counts in descending order
 * @param {Object} typeCounts - Object with type keys and count values
 * @returns {Array} Array of [type, count] pairs sorted descending
 */
function sortTypeCountsDescending(typeCounts) {
    return Object.entries(typeCounts)
        .sort((a, b) => b[1] - a[1]);
}

/**
 * Get chart colour for a node type
 * @param {string} type - Node type
 * @returns {string} Colour hex code
 */
function getChartColorForType(type) {
    return nodeTypeColors[type] || nodeTypeColors.default;
}

/**
 * Render the node type statistics chart
 * @param {Object} typeCounts - Object with type keys and count values
 */
function renderNodeTypeChart(typeCounts) {
    const canvas = document.getElementById('node-type-chart');
    if (!canvas) return;

    // Destroy existing chart
    if (nodeTypeChartState.chartInstance) {
        nodeTypeChartState.chartInstance.destroy();
    }

    const sortedTypes = sortTypeCountsDescending(typeCounts);
    const labels = sortedTypes.map(([type]) => type);
    const data = sortedTypes.map(([, count]) => count);
    const colors = labels.map(type => getChartColorForType(type));

    // Create chart
    nodeTypeChartState.chartInstance = new Chart(canvas, {
        type: 'bar',
        data: {
            labels,
            datasets: [{
                label: 'Node Count',
                data,
                backgroundColor: colors,
                borderColor: colors.map(c => c),
                borderWidth: 1,
                borderRadius: 4,
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            indexAxis: 'y',  // Horizontal bar chart
            plugins: {
                legend: {
                    display: false,
                },
                tooltip: {
                    callbacks: {
                        label: (context) => `${context.parsed.x} nodes`
                    }
                }
            },
            scales: {
                x: {
                    beginAtZero: true,
                    grid: {
                        color: 'rgba(100, 116, 139, 0.2)',
                    },
                    ticks: {
                        color: 'var(--text-secondary, #94a3b8)',
                        stepSize: 1,
                    }
                },
                y: {
                    grid: {
                        display: false,
                    },
                    ticks: {
                        color: 'var(--text-secondary, #94a3b8)',
                    }
                }
            }
        }
    });

    nodeTypeChartState.typeCounts = typeCounts;
}

/**
 * Update node type statistics chart with current graph data
 */
function updateNodeTypeChart() {
    if (!graphState.nodes || graphState.nodes.length === 0) return;

    const typeCounts = countNodesByType(graphState.nodes);
    renderNodeTypeChart(typeCounts);
}

// ============================================
// Update Stats Panel Integration
// ============================================

/**
 * Update full graph statistics panel (extended for Task 7.2)
 * Overrides/extends updateGraphStatsPanel
 */
const originalUpdateGraphStatsPanel = typeof updateGraphStatsPanel !== 'undefined'
    ? updateGraphStatsPanel
    : function() {};

// Override updateGraphStatsPanel to include node type chart
function updateGraphStatsPanelExtended() {
    // Call original function for basic stats
    const nodeCountEl = document.getElementById('graph-node-count');
    const edgeCountEl = document.getElementById('graph-edge-count');
    const depthEl = document.getElementById('graph-depth');
    const generatedAtEl = document.getElementById('graph-generated-at');

    if (nodeCountEl) nodeCountEl.textContent = graphState.metadata.node_count || 0;
    if (edgeCountEl) edgeCountEl.textContent = graphState.metadata.edge_count || 0;
    if (depthEl) depthEl.textContent = graphState.metadata.depth || 0;
    if (generatedAtEl) {
        const date = graphState.metadata.generated_at
            ? new Date(graphState.metadata.generated_at).toLocaleString()
            : 'N/A';
        generatedAtEl.textContent = date;
    }

    // Task 7.2: Update node type chart
    updateNodeTypeChart();

    // Task 6.3: Recompute sensitivity paths
    if (graphState.nodes.length > 0) {
        sensitivityPathState.paths = findAllSensitivityPaths(graphState.nodes, graphState.links);
        if (sensitivityPathState.isEnabled) {
            updateSensitivityPathSelector();
            if (sensitivityPathState.paths.length > 0 && !sensitivityPathState.highlightedPath) {
                highlightSensitivityPath(sensitivityPathState.paths[0]);
            }
        }
    }
}

// Apply extended stats panel update
// Note: This is called from renderGraph()

// ============================================
// Unit Tests for Graph Functionality
// ============================================

/**
 * Run unit tests for GraphManager and Graph visualisation
 * Can be triggered from browser console: runGraphTests()
 */
function runGraphTests() {
    const results = [];
    const assert = (condition, message) => {
        results.push({ passed: condition, message });
        if (!condition) {
            console.error(`FAIL: ${message}`);
        } else {
            console.log(`PASS: ${message}`);
        }
    };

    console.log('=== GraphManager Tests ===');

    // Test 1: GraphManager instantiation
    const gm = new GraphManager();
    assert(gm !== null, 'GraphManager instantiation');
    assert(Object.keys(gm.graphs).length === 0, 'GraphManager has empty graphs initially');
    assert(gm.subscriptions.size === 0, 'GraphManager has empty subscriptions initially');

    // Test 2: Subscribe/Unsubscribe
    gm.subscribe('T001');
    assert(gm.isSubscribed('T001'), 'Subscribe adds trade to subscriptions');
    assert(!gm.isSubscribed('T002'), 'Non-subscribed trade returns false');

    gm.unsubscribe('T001');
    assert(!gm.isSubscribed('T001'), 'Unsubscribe removes trade from subscriptions');

    // Test 3: Duplicate subscription is idempotent
    gm.subscribe('T001');
    gm.subscribe('T001');
    gm.subscribe('T001');
    assert(gm.subscriptions.size === 1, 'Duplicate subscriptions are idempotent');

    // Test 4: Listener management
    let callbackCalled = false;
    const testCallback = () => { callbackCalled = true; };
    gm.addListener('test_event', testCallback);
    gm.notifyListeners('test_event', {});
    assert(callbackCalled, 'Listener callback is called');

    // Test 5: handleGraphUpdate only processes subscribed trades
    gm.clearCache();
    gm.subscriptions.clear();
    gm.graphs['T001'] = {
        nodes: [{ id: 'N1', value: 100 }],
        links: [],
        metadata: {}
    };

    gm.subscribe('T001');
    gm.handleGraphUpdate({
        type: 'graph_update',
        data: {
            trade_id: 'T001',
            updated_nodes: [{ id: 'N1', value: 150 }]
        }
    });
    assert(gm.graphs['T001'].nodes[0].value === 150, 'handleGraphUpdate updates node values');

    // Test 6: handleGraphUpdate ignores non-subscribed trades
    gm.handleGraphUpdate({
        type: 'graph_update',
        data: {
            trade_id: 'T002',
            updated_nodes: [{ id: 'N1', value: 200 }]
        }
    });
    assert(gm.graphs['T001'].nodes[0].value === 150, 'handleGraphUpdate ignores non-subscribed trades');

    // Test 7: Node colour mapping
    assert(getNodeColor({ group: 'input', is_sensitivity_target: false }) === '#3b82f6', 'Input nodes are blue');
    assert(getNodeColor({ group: 'intermediate', is_sensitivity_target: false }) === '#6b7280', 'Intermediate nodes are grey');
    assert(getNodeColor({ group: 'output', is_sensitivity_target: false }) === '#22c55e', 'Output nodes are green');
    assert(getNodeColor({ group: 'input', is_sensitivity_target: true }) === '#f97316', 'Sensitivity targets are orange');

    console.log('=== Test Results ===');
    const passed = results.filter(r => r.passed).length;
    const total = results.length;
    console.log(`${passed}/${total} tests passed`);

    return { passed, total, results };
}

// ============================================
// Task 6.2: Node Search and Highlight
// ============================================

/**
 * Search state for graph node search functionality
 */
const graphSearchState = {
    query: '',
    results: [],
    currentIndex: -1,
    debounceTimer: null,
};

/**
 * Search nodes by label, id, or type
 * @param {string} query - Search query
 * @returns {Array} Array of matching nodes
 */
function searchNodes(query) {
    if (!query || query.trim() === '') {
        return [];
    }

    const normalizedQuery = query.toLowerCase().trim();

    return graphState.nodes.filter(node => {
        // Search by label
        if (node.label && node.label.toLowerCase().includes(normalizedQuery)) {
            return true;
        }
        // Search by id
        if (node.id && node.id.toLowerCase().includes(normalizedQuery)) {
            return true;
        }
        // Search by node type
        if (node.node_type && node.node_type.toLowerCase().includes(normalizedQuery)) {
            return true;
        }
        // Search by group
        if (node.group && node.group.toLowerCase().includes(normalizedQuery)) {
            return true;
        }
        return false;
    });
}

/**
 * Highlight search results on the graph
 * @param {Array} matchingNodes - Array of matching node objects
 */
function highlightSearchResults(matchingNodes) {
    if (!graphState.g) return;

    const matchingIds = new Set(matchingNodes.map(n => n.id));

    if (matchingNodes.length === 0) {
        // Clear all highlights - restore normal state
        graphState.g.selectAll('.node-group')
            .classed('node-dimmed', false)
            .select('circle')
            .classed('node-highlight-search', false);

        graphState.g.selectAll('.link')
            .classed('link-dimmed', false);

        return;
    }

    // Dim non-matching nodes
    graphState.g.selectAll('.node-group')
        .classed('node-dimmed', d => !matchingIds.has(d.id))
        .select('circle')
        .classed('node-highlight-search', d => matchingIds.has(d.id));

    // Dim non-matching links
    graphState.g.selectAll('.link')
        .classed('link-dimmed', d => {
            const sourceId = typeof d.source === 'object' ? d.source.id : d.source;
            const targetId = typeof d.target === 'object' ? d.target.id : d.target;
            return !matchingIds.has(sourceId) && !matchingIds.has(targetId);
        });
}

/**
 * Focus the graph view on a specific node
 * @param {string} nodeId - The ID of the node to focus on
 */
function focusOnNode(nodeId) {
    if (!graphState.svg || !graphState.zoom || !graphState.g) return;

    const node = graphState.nodes.find(n => n.id === nodeId);
    if (!node || node.x === undefined || node.y === undefined) return;

    // Get the SVG dimensions
    const svg = graphState.svg.node();
    const svgRect = svg.getBoundingClientRect();
    const width = svgRect.width || 800;
    const height = svgRect.height || 600;

    // Calculate the transform to center on the node
    const scale = 1.5; // Zoom in slightly
    const x = width / 2 - node.x * scale;
    const y = height / 2 - node.y * scale;

    // Apply transform with animation
    graphState.svg.transition()
        .duration(500)
        .call(
            graphState.zoom.transform,
            d3.zoomIdentity.translate(x, y).scale(scale)
        );

    // Briefly highlight the focused node
    graphState.g.selectAll('.node-group')
        .filter(d => d.id === nodeId)
        .select('circle')
        .transition()
        .duration(200)
        .attr('r', d => (d.is_sensitivity_target ? 16 : 12))
        .transition()
        .duration(300)
        .attr('r', d => (d.is_sensitivity_target ? 12 : 8));
}

/**
 * Perform search and update UI
 * @param {string} query - Search query
 */
function performGraphSearch(query) {
    graphSearchState.query = query;
    graphSearchState.results = searchNodes(query);
    graphSearchState.currentIndex = graphSearchState.results.length > 0 ? 0 : -1;

    // Update UI elements
    const clearBtn = document.getElementById('graph-search-clear');
    const resultsPanel = document.getElementById('graph-search-results');
    const resultsCount = document.getElementById('search-results-count');
    const prevBtn = document.getElementById('search-prev');
    const nextBtn = document.getElementById('search-next');

    // Show/hide clear button
    if (clearBtn) {
        clearBtn.style.display = query ? 'flex' : 'none';
    }

    // Update results panel
    if (resultsPanel && resultsCount) {
        if (query && graphSearchState.results.length > 0) {
            resultsPanel.style.display = 'block';
            resultsCount.textContent = `${graphSearchState.results.length} result${graphSearchState.results.length !== 1 ? 's' : ''}`;

            // Enable/disable navigation buttons
            if (prevBtn) prevBtn.disabled = graphSearchState.results.length <= 1;
            if (nextBtn) nextBtn.disabled = graphSearchState.results.length <= 1;
        } else if (query) {
            resultsPanel.style.display = 'block';
            resultsCount.textContent = 'No results';
            if (prevBtn) prevBtn.disabled = true;
            if (nextBtn) nextBtn.disabled = true;
        } else {
            resultsPanel.style.display = 'none';
        }
    }

    // Highlight matching nodes
    highlightSearchResults(graphSearchState.results);

    // Focus on first result if any
    if (graphSearchState.currentIndex >= 0) {
        focusOnNode(graphSearchState.results[graphSearchState.currentIndex].id);
    }
}

/**
 * Navigate to next search result
 */
function nextSearchResult() {
    if (graphSearchState.results.length === 0) return;

    graphSearchState.currentIndex =
        (graphSearchState.currentIndex + 1) % graphSearchState.results.length;
    focusOnNode(graphSearchState.results[graphSearchState.currentIndex].id);
}

/**
 * Navigate to previous search result
 */
function prevSearchResult() {
    if (graphSearchState.results.length === 0) return;

    graphSearchState.currentIndex =
        (graphSearchState.currentIndex - 1 + graphSearchState.results.length) %
        graphSearchState.results.length;
    focusOnNode(graphSearchState.results[graphSearchState.currentIndex].id);
}

/**
 * Clear search and reset UI
 */
function clearGraphSearch() {
    graphSearchState.query = '';
    graphSearchState.results = [];
    graphSearchState.currentIndex = -1;

    const searchInput = document.getElementById('graph-search-input');
    const clearBtn = document.getElementById('graph-search-clear');
    const resultsPanel = document.getElementById('graph-search-results');

    if (searchInput) searchInput.value = '';
    if (clearBtn) clearBtn.style.display = 'none';
    if (resultsPanel) resultsPanel.style.display = 'none';

    // Clear highlights
    highlightSearchResults([]);
}

/**
 * Initialise search controls event listeners
 */
function initSearchControls() {
    const searchInput = document.getElementById('graph-search-input');
    const clearBtn = document.getElementById('graph-search-clear');
    const prevBtn = document.getElementById('search-prev');
    const nextBtn = document.getElementById('search-next');

    // Search input with debounce
    if (searchInput) {
        searchInput.addEventListener('input', (e) => {
            const query = e.target.value;

            // Clear existing debounce timer
            if (graphSearchState.debounceTimer) {
                clearTimeout(graphSearchState.debounceTimer);
            }

            // Debounce search for 200ms
            graphSearchState.debounceTimer = setTimeout(() => {
                performGraphSearch(query);
            }, 200);
        });

        // Handle Enter key to navigate to next result
        searchInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                if (e.shiftKey) {
                    prevSearchResult();
                } else {
                    nextSearchResult();
                }
            } else if (e.key === 'Escape') {
                clearGraphSearch();
                searchInput.blur();
            }
        });
    }

    // Clear button
    if (clearBtn) {
        clearBtn.addEventListener('click', clearGraphSearch);
    }

    // Navigation buttons
    if (prevBtn) {
        prevBtn.addEventListener('click', prevSearchResult);
    }
    if (nextBtn) {
        nextBtn.addEventListener('click', nextSearchResult);
    }
}

/**
 * Run unit tests for search functionality
 * Can be triggered from browser console: runSearchTests()
 */
function runSearchTests() {
    const results = [];
    const assert = (condition, message) => {
        results.push({ passed: condition, message });
        if (!condition) {
            console.error(`FAIL: ${message}`);
        } else {
            console.log(`PASS: ${message}`);
        }
    };

    console.log('=== Graph Search Tests ===');

    // Setup test data
    const testNodes = [
        { id: 'N1', label: 'spot', node_type: 'input', group: 'input' },
        { id: 'N2', label: 'volatility', node_type: 'input', group: 'input' },
        { id: 'N3', label: 'spot * vol', node_type: 'mul', group: 'intermediate' },
        { id: 'N4', label: 'price', node_type: 'output', group: 'output' },
        { id: 'N5', label: 'delta', node_type: 'output', group: 'sensitivity' },
    ];

    // Backup original state
    const originalNodes = graphState.nodes;
    graphState.nodes = testNodes;

    // Test 1: Search by label
    let found = searchNodes('spot');
    assert(found.length === 2, 'Search "spot" finds 2 nodes (spot, spot * vol)');
    assert(found.some(n => n.id === 'N1'), 'Search "spot" includes N1');
    assert(found.some(n => n.id === 'N3'), 'Search "spot" includes N3');

    // Test 2: Search by id
    found = searchNodes('N2');
    assert(found.length === 1, 'Search "N2" finds 1 node');
    assert(found[0].id === 'N2', 'Search "N2" finds correct node');

    // Test 3: Search by node type
    found = searchNodes('input');
    assert(found.length === 2, 'Search "input" finds 2 nodes');

    // Test 4: Search by group
    found = searchNodes('output');
    assert(found.length === 2, 'Search "output" finds 2 nodes (output group)');

    // Test 5: Case insensitive search
    found = searchNodes('SPOT');
    assert(found.length === 2, 'Search is case insensitive');

    // Test 6: Empty query
    found = searchNodes('');
    assert(found.length === 0, 'Empty query returns no results');

    // Test 7: No matches
    found = searchNodes('nonexistent');
    assert(found.length === 0, 'Non-matching query returns no results');

    // Test 8: Whitespace handling
    found = searchNodes('  spot  ');
    assert(found.length === 2, 'Whitespace is trimmed from query');

    // Restore original state
    graphState.nodes = originalNodes;

    console.log('=== Search Test Results ===');
    const passed = results.filter(r => r.passed).length;
    const total = results.length;
    console.log(`${passed}/${total} tests passed`);

    return { passed, total, results };
}

// ============================================
// Task 6.3: Sensitivity Path Tests
// ============================================

/**
 * Run unit tests for Sensitivity Path functionality
 * Can be triggered from browser console: runSensitivityPathTests()
 */
function runSensitivityPathTests() {
    const results = [];
    const assert = (condition, message) => {
        results.push({ passed: condition, message });
        if (!condition) {
            console.error(`FAIL: ${message}`);
        } else {
            console.log(`PASS: ${message}`);
        }
    };

    console.log('=== Sensitivity Path Tests ===');

    // Test graph data
    const testNodes = [
        { id: 'N1', type: 'input', label: 'spot', group: 'input', is_sensitivity_target: true },
        { id: 'N2', type: 'input', label: 'vol', group: 'input', is_sensitivity_target: true },
        { id: 'N3', type: 'mul', label: 'spot * vol', group: 'intermediate', is_sensitivity_target: false },
        { id: 'N4', type: 'add', label: 'sum', group: 'intermediate', is_sensitivity_target: false },
        { id: 'N5', type: 'output', label: 'price', group: 'output', is_sensitivity_target: false },
    ];
    const testLinks = [
        { source: 'N1', target: 'N3' },
        { source: 'N2', target: 'N3' },
        { source: 'N3', target: 'N4' },
        { source: 'N4', target: 'N5' },
    ];

    // Test 1: Find sensitivity target nodes
    const sensitivityTargets = findSensitivityTargets(testNodes);
    assert(sensitivityTargets.length === 2, 'findSensitivityTargets finds all targets');
    assert(sensitivityTargets.includes('N1'), 'findSensitivityTargets includes N1');
    assert(sensitivityTargets.includes('N2'), 'findSensitivityTargets includes N2');

    // Test 2: Find output nodes
    const outputNodes = findOutputNodes(testNodes);
    assert(outputNodes.length === 1, 'findOutputNodes finds output node');
    assert(outputNodes[0] === 'N5', 'findOutputNodes returns N5');

    // Test 3: Build adjacency list
    const adjacency = buildAdjacencyList(testLinks);
    assert(adjacency['N1'].includes('N3'), 'Adjacency list has N1->N3');
    assert(adjacency['N3'].includes('N4'), 'Adjacency list has N3->N4');
    assert(adjacency['N4'].includes('N5'), 'Adjacency list has N4->N5');

    // Test 4: Find path from sensitivity target to output
    const path = findPathBFS('N1', 'N5', adjacency);
    assert(path !== null, 'findPathBFS finds path from N1 to N5');
    assert(path.length === 4, 'Path length is 4 (N1->N3->N4->N5)');
    assert(path[0] === 'N1', 'Path starts with N1');
    assert(path[path.length - 1] === 'N5', 'Path ends with N5');

    // Test 5: Find all sensitivity paths
    const allPaths = findAllSensitivityPaths(testNodes, testLinks);
    assert(allPaths.length === 2, 'findAllSensitivityPaths returns 2 paths');
    assert(allPaths[0].from === 'N1' || allPaths[0].from === 'N2', 'First path starts from sensitivity target');

    // Test 6: No path when disconnected
    const disconnectedNodes = [
        { id: 'D1', type: 'input', group: 'input', is_sensitivity_target: true },
        { id: 'D2', type: 'output', group: 'output', is_sensitivity_target: false },
    ];
    const disconnectedPaths = findAllSensitivityPaths(disconnectedNodes, []);
    assert(disconnectedPaths.length === 0, 'No paths when graph is disconnected');

    // Test 7: Get edges for path
    const pathEdges = getEdgesForPath(path, testLinks);
    assert(pathEdges.length === 3, 'getEdgesForPath returns 3 edges for 4-node path');

    console.log('=== Sensitivity Path Test Results ===');
    const passed = results.filter(r => r.passed).length;
    const total = results.length;
    console.log(`${passed}/${total} tests passed`);

    return { passed, total, results };
}

// ============================================
// Task 7.2: Node Type Statistics Tests
// ============================================

/**
 * Run unit tests for Node Type Statistics Chart functionality
 * Can be triggered from browser console: runNodeTypeChartTests()
 */
function runNodeTypeChartTests() {
    const results = [];
    const assert = (condition, message) => {
        results.push({ passed: condition, message });
        if (!condition) {
            console.error(`FAIL: ${message}`);
        } else {
            console.log(`PASS: ${message}`);
        }
    };

    console.log('=== Node Type Statistics Tests ===');

    // Test nodes
    const testNodes = [
        { id: 'N1', type: 'input' },
        { id: 'N2', type: 'input' },
        { id: 'N3', type: 'mul' },
        { id: 'N4', type: 'mul' },
        { id: 'N5', type: 'mul' },
        { id: 'N6', type: 'add' },
        { id: 'N7', type: 'exp' },
        { id: 'N8', type: 'output' },
    ];

    // Test 1: Count nodes by type
    const typeCounts = countNodesByType(testNodes);
    assert(typeCounts.input === 2, 'countNodesByType counts input nodes');
    assert(typeCounts.mul === 3, 'countNodesByType counts mul nodes');
    assert(typeCounts.add === 1, 'countNodesByType counts add nodes');
    assert(typeCounts.exp === 1, 'countNodesByType counts exp nodes');
    assert(typeCounts.output === 1, 'countNodesByType counts output nodes');

    // Test 2: Empty nodes
    const emptyTypeCounts = countNodesByType([]);
    assert(Object.keys(emptyTypeCounts).length === 0, 'countNodesByType handles empty array');

    // Test 3: Sort type counts descending
    const sortedTypes = sortTypeCountsDescending(typeCounts);
    assert(sortedTypes[0][0] === 'mul', 'sortTypeCountsDescending puts mul first');
    assert(sortedTypes[0][1] === 3, 'sortTypeCountsDescending mul count is 3');
    assert(sortedTypes.length === 5, 'sortTypeCountsDescending returns all types');

    // Test 4: Get chart colour for type
    assert(getChartColorForType('input') !== undefined, 'getChartColorForType returns colour for input');
    assert(getChartColorForType('add') !== undefined, 'getChartColorForType returns colour for add');
    assert(getChartColorForType('unknown') !== undefined, 'getChartColorForType returns default for unknown');

    console.log('=== Node Type Statistics Test Results ===');
    const passed = results.filter(r => r.passed).length;
    const total = results.length;
    console.log(`${passed}/${total} tests passed`);

    return { passed, total, results };
}
