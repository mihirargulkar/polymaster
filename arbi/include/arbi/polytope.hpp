#pragma once
#include "arbi/common.hpp"
#include <Eigen/Dense>

namespace arbi {

// Result of feasibility check
struct FeasibilityResult {
  bool feasible;        // true = no arbitrage
  double violation;     // magnitude of constraint violation
  Eigen::VectorXd dual; // dual variables (shadow prices)
};

class MarginalPolytope {
public:
  MarginalPolytope() = default;

  // Build IP constraints from dependency list
  void buildConstraints(size_t num_markets,
                        const std::vector<Dependency> &deps);

  // Check if price vector lies inside the polytope
  FeasibilityResult checkFeasibility(const Eigen::VectorXd &prices);

  // Solve LP: minimize c^T x subject to constraints
  // Returns optimal x, or nullopt if infeasible
  std::optional<Eigen::VectorXd> solveLP(const Eigen::VectorXd &objective);

  size_t numConstraints() const { return num_constraints_; }
  size_t numVariables() const { return num_vars_; }

private:
  size_t num_vars_ = 0;
  size_t num_constraints_ = 0;

  // Constraint matrix Ax <= b  (stored in sparse triplet form for GLPK)
  struct Triplet {
    int row;
    int col;
    double val;
  };
  std::vector<Triplet> A_triplets_;
  std::vector<double> b_upper_;
  std::vector<double> b_lower_;

  // Variable bounds
  std::vector<double> var_lb_;
  std::vector<double> var_ub_;
};

} // namespace arbi
