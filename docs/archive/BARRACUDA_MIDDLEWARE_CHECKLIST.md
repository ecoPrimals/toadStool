# BarraCuda Scientific Middleware — Implementation Checklist

**Sprint**: 3 weeks (15 working days)  
**Goal**: Extract 600 lines from hotSpring L1/L2 binaries into reusable BarraCuda library

---

## Week 1: Core Infrastructure (CRITICAL PATH)

### Day 1: Module Structure
- [ ] Create `crates/barracuda/src/linalg/mod.rs`
- [ ] Create `crates/barracuda/src/surrogate/mod.rs`
- [ ] Create `crates/barracuda/src/optimize/mod.rs`
- [ ] Create `crates/barracuda/src/numerical/mod.rs`
- [ ] Create `crates/barracuda/src/special/mod.rs`
- [ ] Update `crates/barracuda/src/lib.rs` to export new modules
- [ ] `cargo check` passes

### Day 2: Linear Algebra
- [ ] Extract `solve_f64` from L1:450-503, L2:1130-1161
- [ ] Move to `crates/barracuda/src/linalg/solve.rs`
- [ ] Add error handling (`BarracudaError::SingularMatrix`)
- [ ] Write unit tests (random SPD systems vs numpy)
- [ ] Benchmark vs numpy.linalg.solve
- [ ] Document with examples

### Day 3: RBF Surrogate (Part 1)
- [ ] Extract `BarracudaRBFSurrogate` struct from L1:297-446
- [ ] Move to `crates/barracuda/src/surrogate/rbf.rs`
- [ ] Implement `RBFKernel` enum (TPS only for now)
- [ ] Implement `RBFSurrogate::train()`
- [ ] Add unit tests (train on known function)

### Day 4: RBF Surrogate (Part 2) + Nelder-Mead
- [ ] Implement `RBFSurrogate::predict()`
- [ ] Test train + predict pipeline vs scipy.interpolate.RBFInterpolator
- [ ] Extract `nelder_mead` from L1:510-627, L2:1167-1250
- [ ] Move to `crates/barracuda/src/optimize/nelder_mead.rs`
- [ ] Test on Rosenbrock, Rastrigin functions
- [ ] Benchmark convergence vs scipy.optimize.fmin

### Day 5: Integration
- [ ] Update L1 binary to use `barracuda::linalg::solve_f64`
- [ ] Update L1 binary to use `barracuda::surrogate::RBFSurrogate`
- [ ] Update L1 binary to use `barracuda::optimize::nelder_mead`
- [ ] Run L1 validation — **must produce identical results**
- [ ] Update L2 binary similarly
- [ ] Run L2 validation — **must produce identical results**
- [ ] Tag: `v0.2.1-scientific-core`

**Week 1 Success Criteria**:
- ✅ L1 and L2 binaries use library modules
- ✅ Results identical to inline code (validate with .json comparison)
- ✅ All tests passing
- ✅ Zero performance regression

---

## Week 2: Optimization & Sampling (ACCURACY MULTIPLIER)

### Day 6: Root-Finding
- [ ] Extract `bisect` from L2:860-874
- [ ] Move to `crates/barracuda/src/optimize/bisect.rs`
- [ ] Add tests (find √2, roots of polynomials)
- [ ] Document vs scipy.optimize.bisect

### Day 7: Latin Hypercube Sampling
- [ ] Implement `latin_hypercube()` in `optimize/latin_hypercube.rs`
- [ ] Algorithm: Stratified random sampling
- [ ] Test space-filling properties (min pairwise distance)
- [ ] Benchmark vs scipy.stats.qmc.LatinHypercube
- [ ] Document with examples

### Day 8: SparsitySampler (THE PRIZE) — Part 1
- [ ] Study mystic.SparsitySampler algorithm
- [ ] Implement candidate generation
- [ ] Implement maximin distance selection
- [ ] Wire up GPU cdist for distance computation

### Day 9: SparsitySampler — Part 2
- [ ] Complete `sparsity_sampler()` in `optimize/sparsity_sampler.rs`
- [ ] Test gap-filling behavior
- [ ] Benchmark: converge to χ²=2.0 in how many evals?
- [ ] Compare to random sampling

### Day 10: Multi-Start Optimization
- [ ] Implement `multi_start_nelder_mead()` in `optimize/multi_start.rs`
- [ ] Use rayon for parallel starts
- [ ] Use latin_hypercube for starting points
- [ ] Test on Rastrigin (many local minima)
- [ ] Benchmark parallel efficiency

**Week 2 Success Criteria**:
- ✅ `sparsity_sampler` implemented and tested
- ✅ Convergence test: 3× fewer evals than random for same accuracy
- ✅ Multi-start optimizer leverages rayon

---

## Week 3: Numerical Methods & Polish

### Day 11: Numerical Methods
- [ ] Extract `gradient_1d` from L2:834-844
- [ ] Move to `numerical/gradient.rs`
- [ ] Test vs numpy.gradient (3-point stencil)
- [ ] Extract `trapz` from L2:848-857
- [ ] Move to `numerical/integrate.rs`
- [ ] Test vs numpy.trapz
- [ ] Extract `trapz_product` — weighted integration
- [ ] Test on known integrals

### Day 12: Special Functions
- [ ] Extract `gamma_fn` from L2:886-931
- [ ] Move to `special/gamma.rs`
- [ ] Test vs scipy.special.gamma (half-integers + general)
- [ ] Extract `factorial` from L2:877-882
- [ ] Move to `special/factorial.rs`
- [ ] Inline Laguerre from L2:ho_radial
- [ ] Move to `special/laguerre.rs`
- [ ] Test all vs scipy.special

### Day 13: Linear Algebra Extensions
- [ ] Wrap nalgebra::SymmetricEigen as `linalg::eigh()`
- [ ] Test vs numpy.linalg.eigh (eigenvalue sorting)
- [ ] Stub `cholesky_f64()` (future CPU implementation)
- [ ] Stub `forward_substitution()`, `backward_substitution()`
- [ ] Document f64 vs f32 WGSL shaders

### Day 14: RBF Kernel Variants
- [ ] Implement `RBFKernel::Gaussian`
- [ ] Implement `RBFKernel::Multiquadric`
- [ ] Implement `RBFKernel::InverseMultiquadric`
- [ ] Implement `RBFKernel::Cubic`
- [ ] Test all kernels on interpolation problems
- [ ] Document when to use which kernel

### Day 15: Documentation & Release
- [ ] Write `crates/barracuda/src/SCIENTIFIC_COMPUTING.md`
- [ ] Write module-level READMEs (linalg, surrogate, optimize, etc.)
- [ ] Complete API documentation (rustdoc examples for all pub fns)
- [ ] Create tutorial: "RBF Surrogate from Scratch"
- [ ] Update main BarraCuda README
- [ ] Tag: `v0.2.2-scientific-complete`

**Week 3 Success Criteria**:
- ✅ All 5 modules complete and documented
- ✅ 100% pub fn coverage with doc examples
- ✅ >90% test coverage
- ✅ Tutorial validates end-to-end workflow

---

## Final Validation

### Regression Tests
- [ ] L1 binary produces identical results (χ², timing within 5%)
- [ ] L2 binary produces identical results (χ², timing within 5%)
- [ ] RBF predictions match scipy to <1e-10
- [ ] Nelder-Mead convergence matches scipy
- [ ] All special functions match scipy to <1e-14

### Performance Benchmarks
- [ ] RBF train (5k points): <1s (maintain 14× speedup)
- [ ] L1 full run: <10s (maintain 14× speedup)
- [ ] L2 full run: <2100s (maintain 1.7× speedup)
- [ ] Library overhead: <1% (vs inline code)

### Documentation Quality
- [ ] Every pub fn has doc comment
- [ ] Every pub fn has example
- [ ] Module READMEs explain use cases
- [ ] Tutorial is beginner-friendly
- [ ] Links to papers/references where applicable

### Code Quality
- [ ] All clippy warnings resolved
- [ ] All fmt clean
- [ ] No unsafe blocks (unless documented)
- [ ] Error handling complete (no unwrap in pub fns)

---

## Dependencies to Add

```toml
# In crates/barracuda/Cargo.toml

[dependencies]
# Already present
nalgebra = "0.32"
rayon = "1.8"
thiserror = "1.0"

# Add for sampling
rand = "0.8"
rand_distr = "0.4"

[dev-dependencies]
# Add for benchmarks
criterion = "0.5"
approx = "0.5"  # For float comparisons in tests
```

---

## Post-Sprint Tasks

### L3 Integration (Week 4)
- [ ] L3 binary uses library modules
- [ ] Validates API ergonomics
- [ ] Finds any missing functionality

### Optional Enhancements
- [ ] f64 WGSL shaders (feature-gated)
- [ ] Brent's method root-finder
- [ ] CMA-ES optimizer
- [ ] Kriging/GP regression
- [ ] Multi-fidelity surrogates

---

## Risk Monitoring

| Risk | Mitigation | Status |
|------|------------|--------|
| Performance regression | Benchmark every extraction | ⬜ |
| API instability | Start pub(crate), promote after L3 | ⬜ |
| Python parity broken | Validate every function vs scipy/numpy | ⬜ |
| SparsitySampler complexity | Port algorithm incrementally, test at each step | ⬜ |

---

## Success Definition

**DONE** when:
1. ✅ All 5 modules implemented (`linalg`, `surrogate`, `optimize`, `numerical`, `special`)
2. ✅ L1 and L2 binaries use library (not inline code)
3. ✅ Results identical to pre-extraction validation
4. ✅ Test coverage >90%
5. ✅ Documentation complete (every pub fn + tutorial)
6. ✅ Zero performance regression

**Stretch goal**: `sparsity_sampler` reaches Python L2 accuracy (χ²~2.0) in <2000 evals.

---

**Sprint Owner**: ToadStool/BarraCuda Team  
**Stakeholder**: hotSpring (L3 blocked)  
**Start**: TBD  
**End**: Start + 15 working days

**Daily Standup Questions**:
1. What did I extract yesterday?
2. What am I extracting today?
3. Any blockers? (API design, validation failure, etc.)

**Sprint Review**: Demo extracted modules running L1/L2 with identical results.
