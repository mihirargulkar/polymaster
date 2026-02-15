#include "arbi/frank_wolfe.hpp"
#include <chrono>
#include <cmath>
#include <spdlog/spdlog.h>

namespace arbi {

FrankWolfe::Result FrankWolfe::optimize(const Eigen::VectorXd &prices,
                                        MarginalPolytope &polytope,
                                        int max_iters, double tolerance) {
  auto start = std::chrono::steady_clock::now();
  Result result;
  result.converged = false;
  result.iterations = 0;

  constexpr double EPS = 1e-12;
  const int n = prices.size();

  // Clamp market prices
  Eigen::VectorXd p(n);
  for (int i = 0; i < n; i++)
    p[i] = std::max(EPS, std::min(1.0 - EPS, prices[i]));

  // Start from polytope center, NOT from p
  // (gradient of D_KL(p||q) is zero when q = p → false convergence)
  Eigen::VectorXd q = Eigen::VectorXd::Constant(n, 0.5);

  for (int k = 0; k < max_iters; k++) {
    result.iterations = k + 1;

    // ── Gradient of D_KL(p || q) w.r.t. q ──
    // ∂/∂q_i = -p_i/q_i + (1-p_i)/(1-q_i)
    Eigen::VectorXd grad(n);
    for (int i = 0; i < n; i++) {
      double qi = std::max(EPS, std::min(1.0 - EPS, q[i]));
      grad[i] = -p[i] / qi + (1.0 - p[i]) / (1.0 - qi);
    }

    // ── LP: v = argmin_{v ∈ M} <grad, v> ──
    auto v_opt = polytope.solveLP(grad);
    if (!v_opt) {
      spdlog::warn("[FW] LP infeasible at iteration {}", k);
      break;
    }
    Eigen::VectorXd v = *v_opt;

    // ── Duality gap ──
    double gap = grad.dot(q - v);
    if (gap < tolerance) {
      result.converged = true;
      spdlog::debug("[FW] Converged at iter {} (gap={:.2e})", k, gap);
      break;
    }

    // ── Exact line search: minimize D_KL(p || (1-γ)q + γv) ──
    double gamma_lo = 0.0, gamma_hi = 1.0;
    for (int ls = 0; ls < 30; ls++) {
      double g1 = gamma_lo + (gamma_hi - gamma_lo) / 3.0;
      double g2 = gamma_lo + 2.0 * (gamma_hi - gamma_lo) / 3.0;

      auto klAtGamma = [&](double g) -> double {
        double kl_val = 0.0;
        for (int i = 0; i < n; i++) {
          double qi = (1.0 - g) * q[i] + g * v[i];
          qi = std::max(EPS, std::min(1.0 - EPS, qi));
          kl_val += p[i] * std::log(p[i] / qi) +
                    (1.0 - p[i]) * std::log((1.0 - p[i]) / (1.0 - qi));
        }
        return kl_val;
      };

      if (klAtGamma(g1) < klAtGamma(g2))
        gamma_hi = g2;
      else
        gamma_lo = g1;
    }
    double gamma = (gamma_lo + gamma_hi) / 2.0;

    // Update
    q = (1.0 - gamma) * q + gamma * v;
    for (int i = 0; i < n; i++)
      q[i] = std::max(EPS, std::min(1.0 - EPS, q[i]));
  }

  result.optimal = q;
  result.trade_vector = q - prices;

  // ── Profit calculation ──
  // The guaranteed profit from moving market prices p to the nearest
  // arbitrage-free price q* is measured by:
  //   1. KL divergence D_KL(p || q*)  — information-theoretic measure
  //   2. L1 trade surplus — sum of absolute position changes / 2
  // We use max(kl, l1_surplus) as the profit metric.
  double kl = 0.0;
  for (int i = 0; i < n; i++) {
    double qi = std::max(EPS, std::min(1.0 - EPS, q[i]));
    double pi = p[i];
    kl +=
        pi * std::log(pi / qi) + (1.0 - pi) * std::log((1.0 - pi) / (1.0 - qi));
  }

  double l1 = result.trade_vector.lpNorm<1>() * 0.5;
  result.profit = std::max(kl, l1);

  auto end = std::chrono::steady_clock::now();
  result.elapsed_ms =
      std::chrono::duration<double, std::milli>(end - start).count();

  if (result.converged) {
    spdlog::info("[FW] Optimized in {}iters / {:.1f}ms, profit={:.6f}",
                 result.iterations, result.elapsed_ms, result.profit);
  }

  return result;
}

} // namespace arbi
