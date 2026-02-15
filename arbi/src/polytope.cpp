#include "arbi/polytope.hpp"
#include <cmath>
#include <glpk.h>
#include <spdlog/spdlog.h>

namespace arbi {

// ── Build IP constraints from dependencies ──────────────────────────
void MarginalPolytope::buildConstraints(size_t num_markets,
                                        const std::vector<Dependency> &deps) {
  num_vars_ = num_markets;
  A_triplets_.clear();
  b_upper_.clear();
  b_lower_.clear();
  var_lb_.assign(num_markets, 0.0);
  var_ub_.assign(num_markets, 1.0);

  int row = 0;

  for (auto &dep : deps) {
    switch (dep.relation) {
    case Relation::IMPLIES:
      // x_i <= x_j  →  x_i - x_j <= 0
      // A (cause) <= B (effect) in probability
      A_triplets_.push_back({row, (int)dep.market_i, 1.0});
      A_triplets_.push_back({row, (int)dep.market_j, -1.0});
      b_upper_.push_back(0.0);
      b_lower_.push_back(-1e30); // no lower bound
      row++;
      break;

    case Relation::MUTEX:
      // x_i + x_j <= 1
      A_triplets_.push_back({row, (int)dep.market_i, 1.0});
      A_triplets_.push_back({row, (int)dep.market_j, 1.0});
      b_upper_.push_back(1.0);
      b_lower_.push_back(-1e30);
      row++;
      break;

    case Relation::EXACTLY_ONE:
      // x_i + x_j = 1 (encoded as <= 1 and >= 1)
      A_triplets_.push_back({row, (int)dep.market_i, 1.0});
      A_triplets_.push_back({row, (int)dep.market_j, 1.0});
      b_upper_.push_back(1.0);
      b_lower_.push_back(1.0);
      row++;
      break;

    case Relation::INDEPENDENT:
      break;
    }
  }

  // Always add: each price in [0, 1] (handled by variable bounds)
  // Add: YES + NO = 1 for each market pair (if applicable)
  // This is implicitly handled since we work with YES prices

  num_constraints_ = row;
  spdlog::info("[Polytope] Built {} constraints for {} variables",
               num_constraints_, num_vars_);
}

// ── Check feasibility ────────────────────────────────────────────────
FeasibilityResult
MarginalPolytope::checkFeasibility(const Eigen::VectorXd &prices) {

  FeasibilityResult result;
  result.feasible = true;
  result.violation = 0.0;
  result.dual = Eigen::VectorXd::Zero(num_constraints_);

  if (num_constraints_ == 0) {
    // No constraints → always feasible (but no arbitrage detectable)
    return result;
  }

  // Check each constraint manually for the price vector
  // Group triplets by row
  std::vector<double> row_values(num_constraints_, 0.0);

  for (auto &t : A_triplets_) {
    if (t.col < (int)prices.size()) {
      row_values[t.row] += t.val * prices[t.col];
    }
  }

  for (size_t r = 0; r < num_constraints_; r++) {
    double v = row_values[r];

    // Check upper bound violation
    if (v > b_upper_[r] + 1e-9) {
      result.feasible = false;
      double viol = v - b_upper_[r];
      result.violation = std::max(result.violation, viol);
      result.dual[r] = viol;
    }

    // Check lower bound violation
    if (b_lower_[r] > -1e29 && v < b_lower_[r] - 1e-9) {
      result.feasible = false;
      double viol = b_lower_[r] - v;
      result.violation = std::max(result.violation, viol);
      result.dual[r] = -viol;
    }
  }

  return result;
}

// ── Solve LP using GLPK ─────────────────────────────────────────────
std::optional<Eigen::VectorXd>
MarginalPolytope::solveLP(const Eigen::VectorXd &objective) {

  if (num_vars_ == 0)
    return std::nullopt;

  glp_prob *lp = glp_create_prob();
  glp_set_obj_dir(lp, GLP_MIN);

  // Suppress GLPK terminal output
  glp_term_out(GLP_OFF);

  // Add variables (columns)
  glp_add_cols(lp, num_vars_);
  for (size_t j = 0; j < num_vars_; j++) {
    glp_set_col_bnds(lp, j + 1, GLP_DB, var_lb_[j], var_ub_[j]);
    glp_set_obj_coef(lp, j + 1, objective[j]);
  }

  // Add constraints (rows)
  if (num_constraints_ > 0) {
    glp_add_rows(lp, num_constraints_);
    for (size_t r = 0; r < num_constraints_; r++) {
      if (b_lower_[r] > -1e29 && std::abs(b_lower_[r] - b_upper_[r]) < 1e-9) {
        // Equality constraint
        glp_set_row_bnds(lp, r + 1, GLP_FX, b_lower_[r], b_upper_[r]);
      } else if (b_lower_[r] > -1e29) {
        glp_set_row_bnds(lp, r + 1, GLP_DB, b_lower_[r], b_upper_[r]);
      } else {
        glp_set_row_bnds(lp, r + 1, GLP_UP, 0.0, b_upper_[r]);
      }
    }

    // Load constraint matrix (GLPK uses 1-indexed arrays)
    size_t nnz = A_triplets_.size();
    std::vector<int> ia(nnz + 1), ja(nnz + 1);
    std::vector<double> ar(nnz + 1);

    for (size_t k = 0; k < nnz; k++) {
      ia[k + 1] = A_triplets_[k].row + 1; // 1-indexed
      ja[k + 1] = A_triplets_[k].col + 1;
      ar[k + 1] = A_triplets_[k].val;
    }

    glp_load_matrix(lp, nnz, ia.data(), ja.data(), ar.data());
  }

  // Solve
  glp_smcp parm;
  glp_init_smcp(&parm);
  parm.msg_lev = GLP_MSG_OFF;

  int status = glp_simplex(lp, &parm);

  if (status != 0 || glp_get_status(lp) != GLP_OPT) {
    glp_delete_prob(lp);
    return std::nullopt;
  }

  // Extract solution
  Eigen::VectorXd solution(num_vars_);
  for (size_t j = 0; j < num_vars_; j++) {
    solution[j] = glp_get_col_prim(lp, j + 1);
  }

  glp_delete_prob(lp);
  return solution;
}

} // namespace arbi
