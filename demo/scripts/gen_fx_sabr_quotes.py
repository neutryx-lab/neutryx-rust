#!/usr/bin/env python3
"""Generate FX vol demo data from known SABR parameters (beta=1, lognormal).

The output quotes (ATM, RR, BF) are guaranteed to round-trip through the
SABR calibrator because they are derived from the Hagan (2002) implied-vol
formula evaluated at exact delta-derived strikes.
"""

import json
import math
import pathlib
from dataclasses import dataclass
from scipy.stats import norm
from scipy.optimize import brentq

# ──────────────────────────────────────────────────────────────────────
# SABR Hagan (2002) implied vol — lognormal (beta = 1)
# ──────────────────────────────────────────────────────────────────────

def sabr_lognormal_vol(F: float, K: float, T: float,
                       alpha: float, rho: float, nu: float) -> float:
    """Hagan lognormal SABR implied vol (beta = 1)."""
    eps = 1e-12
    if T < eps:
        return alpha

    if abs(F - K) < eps * F:
        # ATM expansion
        term2 = rho * nu * alpha / 4.0
        term3 = (2.0 - 3.0 * rho**2) * nu**2 / 24.0
        return alpha * (1.0 + (term2 + term3) * T)

    log_fk = math.log(F / K)
    z = (nu / alpha) * log_fk
    sqrt_term = math.sqrt(1.0 - 2.0 * rho * z + z**2)
    x_z = math.log((sqrt_term + z - rho) / (1.0 - rho))

    if abs(x_z) < eps:
        z_over_x = 1.0
    else:
        z_over_x = z / x_z

    base = alpha
    term2 = rho * nu * alpha / 4.0
    term3 = (2.0 - 3.0 * rho**2) * nu**2 / 24.0
    expansion = 1.0 + (term2 + term3) * T

    return base * z_over_x * expansion


# ──────────────────────────────────────────────────────────────────────
# BS delta ↔ strike helpers  (spot delta convention for G10)
# ──────────────────────────────────────────────────────────────────────

def bs_call_delta(F: float, K: float, T: float, sigma: float, rf: float) -> float:
    """Spot delta of a European call (Garman-Kohlhagen)."""
    sqrt_T = math.sqrt(T)
    d1 = (math.log(F / K) + 0.5 * sigma**2 * T) / (sigma * sqrt_T)
    return math.exp(-rf * T) * norm.cdf(d1)


def strike_from_call_delta(F: float, T: float, sigma: float, rf: float,
                           target_delta: float) -> float:
    """Invert BS spot-delta to find the strike for a call."""
    d1 = norm.ppf(target_delta * math.exp(rf * T))
    sqrt_T = math.sqrt(T)
    return F * math.exp(-d1 * sigma * sqrt_T + 0.5 * sigma**2 * T)


def find_sabr_strike_for_delta(F: float, T: float, rf: float,
                               alpha: float, rho: float, nu: float,
                               target_delta: float, is_call: bool,
                               tol: float = 1e-10) -> tuple:
    """Find (strike, sabr_vol) at a given BS spot delta using iteration."""
    # Initial guess: use ATM vol
    vol_atm = sabr_lognormal_vol(F, F, T, alpha, rho, nu)
    K = strike_from_call_delta(F, T, vol_atm, rf, target_delta if is_call else 1.0 - target_delta)

    for _ in range(50):
        vol = sabr_lognormal_vol(F, K, T, alpha, rho, nu)
        K_new = strike_from_call_delta(F, T, vol, rf, target_delta if is_call else 1.0 - target_delta)
        if abs(K_new - K) < tol * F:
            return K_new, vol
        K = K_new

    vol = sabr_lognormal_vol(F, K, T, alpha, rho, nu)
    return K, vol


# ──────────────────────────────────────────────────────────────────────
# Quote generation
# ──────────────────────────────────────────────────────────────────────

@dataclass
class TenorSpec:
    label: str
    expiry: float
    alpha: float
    rho: float
    nu: float


def generate_quotes(spot: float, rd: float, rf: float,
                    tenors: list) -> list:
    """Generate FX vol quotes from SABR parameters."""
    quotes = []
    for t in tenors:
        F = spot * math.exp((rd - rf) * t.expiry)
        T = t.expiry
        alpha, rho, nu = t.alpha, t.rho, t.nu

        # ATM vol (at K = F for delta-neutral straddle approximation)
        atm_vol = sabr_lognormal_vol(F, F, T, alpha, rho, nu)

        # 25-delta
        _, vol_25d_call = find_sabr_strike_for_delta(F, T, rf, alpha, rho, nu, 0.25, True)
        _, vol_25d_put  = find_sabr_strike_for_delta(F, T, rf, alpha, rho, nu, 0.25, False)

        # 10-delta
        _, vol_10d_call = find_sabr_strike_for_delta(F, T, rf, alpha, rho, nu, 0.10, True)
        _, vol_10d_put  = find_sabr_strike_for_delta(F, T, rf, alpha, rho, nu, 0.10, False)

        rr_25d = vol_25d_call - vol_25d_put
        bf_25d = 0.5 * (vol_25d_call + vol_25d_put) - atm_vol
        rr_10d = vol_10d_call - vol_10d_put
        bf_10d = 0.5 * (vol_10d_call + vol_10d_put) - atm_vol

        quotes.append({
            "tenor": t.label,
            "expiry": t.expiry,
            "atmVol": round(atm_vol, 6),
            "rr25d": round(rr_25d, 6),
            "bf25d": round(bf_25d, 6),
            "rr10d": round(rr_10d, 6),
            "bf10d": round(bf_10d, 6),
        })

    return quotes


# ──────────────────────────────────────────────────────────────────────
# EURUSD configuration  (beta = 1 fixed)
# ──────────────────────────────────────────────────────────────────────
# Typical G10 lognormal SABR:
#   alpha ≈ ATM vol,  rho slightly negative,  nu ~ 0.3–0.8

EURUSD_TENORS = [
    TenorSpec("1M",  0.0833, alpha=0.0775, rho=-0.20, nu=0.90),
    TenorSpec("2M",  0.1667, alpha=0.0810, rho=-0.21, nu=0.85),
    TenorSpec("3M",  0.25,   alpha=0.0840, rho=-0.22, nu=0.82),
    TenorSpec("6M",  0.5,    alpha=0.0910, rho=-0.23, nu=0.75),
    TenorSpec("9M",  0.75,   alpha=0.0965, rho=-0.24, nu=0.70),
    TenorSpec("1Y",  1.0,    alpha=0.1005, rho=-0.25, nu=0.60),
    TenorSpec("2Y",  2.0,    alpha=0.1060, rho=-0.25, nu=0.45),
]

# ──────────────────────────────────────────────────────────────────────
# USDJPY configuration  (beta = 1 fixed)
# ──────────────────────────────────────────────────────────────────────
# USDJPY has a positive skew (JPY puts more expensive → positive RR)

USDJPY_TENORS = [
    TenorSpec("1M",  0.0833, alpha=0.0940, rho=0.15, nu=0.95),
    TenorSpec("2M",  0.1667, alpha=0.0970, rho=0.16, nu=0.90),
    TenorSpec("3M",  0.25,   alpha=0.0990, rho=0.17, nu=0.85),
    TenorSpec("6M",  0.5,    alpha=0.1040, rho=0.18, nu=0.78),
    TenorSpec("9M",  0.75,   alpha=0.1070, rho=0.19, nu=0.72),
    TenorSpec("1Y",  1.0,    alpha=0.1090, rho=0.20, nu=0.62),
    TenorSpec("2Y",  2.0,    alpha=0.1130, rho=0.20, nu=0.48),
]


def main():
    out_dir = pathlib.Path(__file__).resolve().parent.parent / "data" / "input" / "fxvol"
    out_dir.mkdir(parents=True, exist_ok=True)

    # ── EURUSD ──
    eurusd = {
        "currencyPair": "EURUSD",
        "referenceDate": "2026-01-23",
        "spot": 1.0850,
        "domesticRate": 0.045,
        "foreignRate": 0.035,
        "quotes": generate_quotes(1.0850, 0.045, 0.035, EURUSD_TENORS),
    }
    path = out_dir / "eurusd.json"
    path.write_text(json.dumps(eurusd, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {path}")
    print("  EURUSD quotes:")
    for q in eurusd["quotes"]:
        print(f"    {q['tenor']:>3s}  ATM={q['atmVol']:.4f}  RR25={q['rr25d']:+.4f}  BF25={q['bf25d']:.4f}  RR10={q['rr10d']:+.4f}  BF10={q['bf10d']:.4f}")

    # ── USDJPY ──
    usdjpy = {
        "currencyPair": "USDJPY",
        "referenceDate": "2026-01-23",
        "spot": 148.50,
        "domesticRate": 0.045,
        "foreignRate": 0.005,
        "quotes": generate_quotes(148.50, 0.045, 0.005, USDJPY_TENORS),
    }
    path = out_dir / "usdjpy.json"
    path.write_text(json.dumps(usdjpy, indent=2) + "\n", encoding="utf-8")
    print(f"\nWrote {path}")
    print("  USDJPY quotes:")
    for q in usdjpy["quotes"]:
        print(f"    {q['tenor']:>3s}  ATM={q['atmVol']:.4f}  RR25={q['rr25d']:+.4f}  BF25={q['bf25d']:.4f}  RR10={q['rr10d']:+.4f}  BF10={q['bf10d']:.4f}")


if __name__ == "__main__":
    main()
