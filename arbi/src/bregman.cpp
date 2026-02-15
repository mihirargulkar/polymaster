#include "arbi/bregman.hpp"
#include <cmath>
#include <spdlog/spdlog.h>

namespace arbi {

// ── Clamp to avoid log(0) ────────────────────────────────────────────
Eigen::VectorXd BregmanProjection::clamp(const Eigen::VectorXd &v) {
  Eigen::VectorXd c(v.size());
  for (int i = 0; i < v.size(); i++) {
    c[i] = std::max(EPS, std::min(1.0 - EPS, v[i]));
  }
  return c;
}

// ── KL divergence D_KL(q || p) ───────────────────────────────────────
// Binary KL for independent Bernoulli variables
double BregmanProjection::klDivergence(const Eigen::VectorXd &q,
                                       const Eigen::VectorXd &p) {
  auto qc = clamp(q);
  auto pc = clamp(p);

  double kl = 0.0;
  for (int i = 0; i < qc.size(); i++) {
    kl += qc[i] * std::log(qc[i] / pc[i]) +
          (1.0 - qc[i]) * std::log((1.0 - qc[i]) / (1.0 - pc[i]));
  }
  return kl;
}

// ── KL gradient ──────────────────────────────────────────────────────
// ∇_q D_KL(q || p) = log(q/p) - log((1-q)/(1-p))
Eigen::VectorXd BregmanProjection::klGradient(const Eigen::VectorXd &q,
                                              const Eigen::VectorXd &p) {
  auto qc = clamp(q);
  auto pc = clamp(p);

  Eigen::VectorXd grad(qc.size());
  for (int i = 0; i < qc.size(); i++) {
    grad[i] = std::log(qc[i] / pc[i]) - std::log((1.0 - qc[i]) / (1.0 - pc[i]));
  }
  return grad;
}

// ── Project p onto polytope M ────────────────────────────────────────
//
// We find q* = argmin_{q ∈ M} D_KL(p || q)
//
// This is the "reverse KL" or I-projection.  The key advantage is that
// the gradient ∇_q D_KL(p || q) = -p/q + (1-p)/(1-q) is NON-ZERO at
// q = p when p is infeasible, allowing Frank-Wolfe to make progress.
//
// The forward KL formulation D_KL(q || p) has ∇_q = 0 at q = p, which
// causes immediate false convergence.
//
BregmanProjection::ProjectionResult
BregmanProjection::project(const Eigen::VectorXd &prices,
                           MarginalPolytope &polytope, int max_iters,
                           double tolerance) {
  ProjectionResult result;
  result.converged = false;
  result.iterations = 0;

  auto p = clamp(prices);
  const int n = p.size();

  // Start from center of polytope (0.5, 0.5, ...) to avoid boundary issues
  Eigen::VectorXd q = Eigen::VectorXd::Constant(n, 0.5);

  for (int k = 0; k < max_iters; k++) {
    result.iterations = k + 1;

    // ── Gradient of D_KL(p || q) w.r.t. q ──
    // ∂/∂q_i D_KL(p||q) = -p_i/q_i + (1-p_i)/(1-q_i)
    Eigen::VectorXd grad(n);
    for (int i = 0; i < n; i++) {
      double qi = std::max(EPS, std::min(1.0 - EPS, q[i]));
      grad[i] = -p[i] / qi + (1.0 - p[i]) / (1.0 - qi);
    }

    // ── LP: v = argmin_{v ∈ M} <grad, v> ──
    auto v_opt = polytope.solveLP(grad);
    if (!v_opt) {
      spdlog::warn("[Bregman] LP solve failed at iter {}", k);
      break;
    }
    Eigen::VectorXd v = *v_opt;

    // ── Duality gap ──
    double gap = grad.dot(q - v);

    if (gap < tolerance) {
      result.converged = true;
      break;
    }

    // ── Exact line search: minimize D_KL(p || (1-γ)q + γv) over γ ∈ [0,1] ──
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
    q = clamp(q);
  }

  result.projected = q;
  // Compute final divergence: klDivergence(a,b) = D_KL(a||b), so this is
  // D_KL(p||q*)
  result.divergence = klDivergence(p, q);

  return result;
}

} // namespace arbi
