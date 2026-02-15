#pragma once
#include "arbi/common.hpp"
#include "arbi/polytope.hpp"
#include <Eigen/Dense>

namespace arbi {

class FrankWolfe {
public:
  struct Result {
    Eigen::VectorXd optimal;      // projected point q*
    Eigen::VectorXd trade_vector; // q* - p (what to trade)
    double profit; // guaranteed profit margin (unitless rate, e.g. 0.05 = 5%)
    int iterations;
    bool converged;
    double elapsed_ms;
  };

  FrankWolfe() = default;

  // Run Frank-Wolfe conditional gradient optimization
  // Finds q* = argmin_{q ∈ M} D_KL(p || q)  (I-projection / reverse KL)
  // Uses exact line search (golden-section bisection), starting from polytope
  // center.
  Result optimize(const Eigen::VectorXd &prices, MarginalPolytope &polytope,
                  int max_iters = 150, double tolerance = 1e-8);
};

} // namespace arbi
