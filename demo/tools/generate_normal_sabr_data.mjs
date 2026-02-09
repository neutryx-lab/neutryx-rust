#!/usr/bin/env node
/**
 * Generate demo swaption normal vol data from known Normal SABR parameters.
 *
 * Usage:  node demo/tools/generate_normal_sabr_data.mjs
 * Output: demo/data/input/irvol/usd.json (overwritten)
 *
 * The generated vols are in "percentage" format (0.68 = 68bp), matching
 * the convention used by the calibration service (which divides by 100).
 */

import { writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// =============================================================================
// Normal SABR formula (β = 0, Hagan approximation)
// =============================================================================

/**
 * Compute Normal SABR implied normal vol σ_N(K) using the Hagan approximation.
 *
 * @param {number} alpha - α (normal vol level, decimal, e.g. 0.0068)
 * @param {number} rho   - ρ (correlation, e.g. -0.25)
 * @param {number} nu    - ν (vol-of-vol, e.g. 0.40)
 * @param {number} F     - Forward rate (e.g. 0.04)
 * @param {number} K     - Strike rate (e.g. 0.035)
 * @param {number} T     - Time to expiry in years (e.g. 1.0)
 * @returns {number} Normal implied vol in decimal (e.g. 0.0068)
 */
function normalSabrVol(alpha, rho, nu, F, K, T) {
  if (T <= 0) return alpha;

  const diff = F - K;
  const avgF = (F + K) / 2;

  // Higher-order correction term
  const correction = (rho * nu * alpha) / (4 * avgF) + ((2 - 3 * rho * rho) / 24) * nu * nu;

  // ATM case
  if (Math.abs(diff) < 1e-10) {
    return alpha * (1 + correction * T);
  }

  // General case
  const z = (nu / alpha) * diff;
  const xz = computeXofZ(z, rho);
  const zOverX = Math.abs(xz) < 1e-12 ? 1.0 : z / xz;

  return alpha * zOverX * (1 + correction * T);
}

/**
 * Compute x(z) = ln((√(1 - 2ρz + z²) + z - ρ) / (1 - ρ))
 */
function computeXofZ(z, rho) {
  const disc = 1 - 2 * rho * z + z * z;
  if (disc < 0) return z; // Fallback for extreme params
  const sqrtDisc = Math.sqrt(disc);
  const num = sqrtDisc + z - rho;
  const den = 1 - rho;
  if (num <= 0 || den <= 0) return z; // Fallback
  return Math.log(num / den);
}

// =============================================================================
// Parameter surface definition
// =============================================================================

const EXPIRIES = ['1M', '3M', '6M', '1Y', '2Y', '5Y', '10Y'];
const TENORS   = ['1Y', '2Y', '5Y', '10Y', '30Y'];

/** Expiry label → year fraction */
const EXPIRY_YF = { '1M': 1/12, '3M': 0.25, '6M': 0.5, '1Y': 1, '2Y': 2, '5Y': 5, '10Y': 10 };

/** Forward swap rates (realistic USD level, varying by tenor) */
const FORWARDS = { '1Y': 0.045, '2Y': 0.043, '5Y': 0.040, '10Y': 0.038, '30Y': 0.035 };

/** Smile strike offsets in basis points */
const SMILE_OFFSETS = [-100, -50, 50, 100];

/**
 * Normal SABR parameters per cell.
 *
 * α values are in DECIMAL (e.g. 0.0068 = 68bp).
 * The pattern:
 *   - α decreases with longer expiry (mean reversion)
 *   - α peaks around 5-10Y tenor
 *   - ρ is more negative for longer expiry
 *   - ν decreases with longer expiry
 */
const PARAMS = {
  //           α(1Y)    α(2Y)    α(5Y)    α(10Y)   α(30Y)   ρ        ν
  '1M':  { a: [0.0068, 0.0072, 0.0078, 0.0082, 0.0075], rho: -0.10, nu: 0.45 },
  '3M':  { a: [0.0065, 0.0069, 0.0075, 0.0079, 0.0073], rho: -0.12, nu: 0.42 },
  '6M':  { a: [0.0062, 0.0066, 0.0072, 0.0076, 0.0071], rho: -0.15, nu: 0.40 },
  '1Y':  { a: [0.0058, 0.0062, 0.0068, 0.0072, 0.0068], rho: -0.18, nu: 0.38 },
  '2Y':  { a: [0.0052, 0.0056, 0.0062, 0.0066, 0.0063], rho: -0.22, nu: 0.35 },
  '5Y':  { a: [0.0045, 0.0048, 0.0054, 0.0058, 0.0055], rho: -0.28, nu: 0.30 },
  '10Y': { a: [0.0040, 0.0043, 0.0048, 0.0052, 0.0050], rho: -0.32, nu: 0.25 },
};

// =============================================================================
// Generate quotes
// =============================================================================

const quotes = [];

for (const expiry of EXPIRIES) {
  const T = EXPIRY_YF[expiry];
  const { a: alphas, rho, nu } = PARAMS[expiry];

  for (let ti = 0; ti < TENORS.length; ti++) {
    const tenor = TENORS[ti];
    const alpha = alphas[ti];
    const F = FORWARDS[tenor];

    // ATM vol (in decimal, then convert to percentage format)
    const atmVolDecimal = normalSabrVol(alpha, rho, nu, F, F, T);
    const atmVol = round(atmVolDecimal * 100, 4); // decimal → percentage (0.68)

    // Smile points
    const smile = SMILE_OFFSETS.map(offsetBp => {
      const K = F + offsetBp / 10000;
      const volDecimal = normalSabrVol(alpha, rho, nu, F, K, T);
      return {
        strikeOffsetBp: offsetBp,
        vol: round(volDecimal * 100, 4), // decimal → percentage
      };
    });

    quotes.push({
      expiry,
      tenor,
      atmVol,
      volType: 'normal',
      smile,
    });
  }
}

// =============================================================================
// Build and write JSON
// =============================================================================

const output = {
  metadata: {
    currency: 'USD',
    volType: 'normal',
    description: 'USD Swaption Normal Volatility Surface with Smile (generated from Normal SABR parameters)',
    source: 'Demo Data (Normal SABR generated)',
    lastUpdated: '2026-01-26T00:00:00Z',
  },
  quotes,
  sabrParameters: {
    beta: 0,
    description: 'Normal SABR parameters used to generate this surface',
    cells: {},
  },
};

// Store the generating parameters for reference
for (const expiry of EXPIRIES) {
  const { a: alphas, rho, nu } = PARAMS[expiry];
  for (let ti = 0; ti < TENORS.length; ti++) {
    const tenor = TENORS[ti];
    const key = `${expiry}|${tenor}`;
    output.sabrParameters.cells[key] = {
      alpha: alphas[ti],
      beta: 0,
      rho,
      nu,
      forward: FORWARDS[tenor],
    };
  }
}

const outPath = resolve(__dirname, '../data/input/irvol/usd.json');
writeFileSync(outPath, JSON.stringify(output, null, 2) + '\n');

console.log(`Generated ${quotes.length} quotes to ${outPath}`);
console.log('\nSample ATM vols (percentage format):');
for (const q of quotes.slice(0, 5)) {
  console.log(`  ${q.expiry} × ${q.tenor}: ATM = ${q.atmVol}  smile = [${q.smile.map(s => s.vol).join(', ')}]`);
}

// Print the parameter table for verification
console.log('\nParameter table (α in bp):');
console.log('Expiry | ' + TENORS.map(t => t.padStart(6)).join(' | '));
console.log('-'.repeat(50));
for (const expiry of EXPIRIES) {
  const { a: alphas } = PARAMS[expiry];
  console.log(`${expiry.padEnd(6)} | ${alphas.map(a => (a * 10000).toFixed(1).padStart(6)).join(' | ')}`);
}

function round(x, dp) {
  const m = Math.pow(10, dp);
  return Math.round(x * m) / m;
}
