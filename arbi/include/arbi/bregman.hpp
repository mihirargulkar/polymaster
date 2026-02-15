#pragma once
#include "arbi/common.hpp"
#include "arbi/polytope.hpp"
#include <Eigen/Dense>

namespace arbi {

class BregmanProjection {
public:
  BregmanProjection() = default;

  // KL divergence D_KL(q || p) = Σ q_i * log(q_i / p_i) + (1-q_i) *
  // log((1-q_i)/(1-p_i)) Note: the first argument is the "numerator"
  // distribution.
  static double klDivergence(const Eigen::VectorXd &q,
                             const Eigen::VectorXd &p);

  // Gradient of KL divergence D_KL(q||p) w.r.t. q:
  //   ∇_q = log(q/p) - log((1-q)/(1-p))
  static Eigen::VectorXd klGradient(const Eigen::VectorXd &q,
                                    const Eigen::VectorXd &p);

  // Project p onto the marginal polytope M using I-projection (reverse KL):
  //   q* = argmin_{q ∈ M} D_KL(p || q)
  //
  // Uses Frank-Wolfe with exact line search, starting from polytope center.
  struct ProjectionResult {
    Eigen::VectorXd projected; // q*
    double divergence;         // D_KL(p || q*)
    int iterations;
    bool converged;
  };

  ProjectionResult project(const Eigen::VectorXd &prices,
                           MarginalPolytope &polytope, int max_iters = 150,
                           double tolerance = 1e-8);

private:
  // Clamp values to avoid log(0)
  static constexpr double EPS = 1e-12;
  static Eigen::VectorXd clamp(const Eigen::VectorXd &v);
};

} // namespace arbi
