# Architecture Decision Records (ADRs)

**Purpose**: Document significant architectural decisions made in ToadStool/BarraCuda

---

## What are ADRs?

Architecture Decision Records (ADRs) capture important architectural decisions along with their context and consequences.

**Format**: Each ADR answers:
1. **What** decision was made
2. **Why** we made it (context, drivers)
3. **What alternatives** we considered
4. **What consequences** (positive, negative, neutral)

---

## Index of ADRs

### ✅ Accepted

| ADR | Title | Date | Impact |
|-----|-------|------|--------|
| [ADR-001](./ADR-001-wgpu-over-opencl-cuda.md) | Use wgpu for GPU Abstraction | 2026-02-05 | **High** - Foundation of GPU strategy |
| [ADR-002](./ADR-002-feature-gate-tpu-support.md) | Feature-Gate TPU Support | 2026-02-05 | **Medium** - Optional hardware pattern |
| [ADR-003](./ADR-003-ntt-for-fhe-polynomial-multiplication.md) | Use NTT for FHE Multiplication | 2026-02-05 | **High** - 56x FHE speedup |
| [ADR-004](./ADR-004-capability-based-service-discovery.md) | Capability-Based Service Discovery | 2026-02-05 | **Critical** - Distributed architecture foundation |

### 🔄 Proposed

| ADR | Title | Status |
|-----|-------|--------|
| ADR-005 | Async Runtime Selection (tokio) | Planned |
| ADR-006 | Error Handling Strategy (thiserror/anyhow) | Planned |

### ❌ Deprecated

| ADR | Title | Superseded By |
|-----|-------|---------------|
| - | - | - |

---

## How to Use ADRs

### Reading ADRs

**For Developers**:
- Read ADRs to understand "why" decisions were made
- Use as reference when making similar decisions
- Check ADRs before proposing alternatives

**For New Team Members**:
- Read ADRs to understand architecture
- ADRs explain trade-offs and context
- Faster onboarding than reading all code

**For Managers**:
- ADRs document technical strategy
- Show decision-making process
- Track evolution of architecture

### Writing New ADRs

**When to Write an ADR**:
- ✅ Significant architectural decision
- ✅ Decision affects multiple components
- ✅ Decision has long-term consequences
- ✅ Trade-offs between multiple options
- ❌ Small implementation details
- ❌ Obvious decisions
- ❌ Temporary workarounds

**Template**:
```markdown
# ADR-XXX: [Title]

**Status**: Proposed | Accepted | Deprecated  
**Date**: YYYY-MM-DD  
**Deciders**: [Who made the decision]  
**Technical Story**: [Context/issue]

## Context and Problem Statement
[What problem are we solving?]

## Decision Drivers
[What factors influenced the decision?]

## Considered Options
[What alternatives did we evaluate?]

## Decision Outcome
[What did we choose and why?]

## Consequences
[What are the positive, negative, and neutral outcomes?]

## Validation
[How did we verify this was the right choice?]
```

---

## ADR Lifecycle

### 1. Proposed
- Initial draft created
- Under discussion
- May be rejected or revised

### 2. Accepted
- Decision has been made
- Implementation may begin
- ADR is reference

### 3. Deprecated
- Decision has been superseded
- Document kept for history
- Link to replacement ADR

---

## Best Practices

### DO ✅

- ✅ Write ADRs before implementing
- ✅ Include benchmarks/data when available
- ✅ Consider multiple alternatives
- ✅ Document consequences honestly
- ✅ Keep ADRs concise (< 2000 words)
- ✅ Link to related ADRs

### DON'T ❌

- ❌ Write ADRs after the fact (document decisions, not history)
- ❌ Skip alternatives section
- ❌ Hide negative consequences
- ❌ Write novel-length ADRs (> 5000 words)
- ❌ Update ADRs after acceptance (create new ADR instead)

---

## Why We Use ADRs

### Benefits

**Knowledge Preservation**:
- Captures "why" behind decisions
- Prevents re-litigating old decisions
- Helps onboard new team members

**Better Decisions**:
- Forces consideration of alternatives
- Documents trade-offs explicitly
- Provides framework for discussion

**Accountability**:
- Clear decision ownership
- Traceable decision history
- Shows evolution of architecture

### Examples of Good ADR Topics

- Choice of database (PostgreSQL vs MongoDB)
- Choice of communication protocol (gRPC vs REST)
- Choice of GPU abstraction (wgpu vs CUDA)
- Choice of encryption library (ring vs rust-crypto)
- Choice of async runtime (tokio vs async-std)

### Examples of Bad ADR Topics

- Variable naming convention (too small)
- Temporary debugging approach (not architectural)
- Personal preference (needs objective reasoning)
- Obvious choices (no real alternatives)

---

## Quick Reference

**Current Count**: 1 ADR (1 accepted, 0 proposed, 0 deprecated)

**Last Updated**: February 5, 2026

**Maintainer**: ToadStool/BarraCuda Core Team

**Template Location**: See "Writing New ADRs" section above

**Questions?**: Open an issue or discuss in team chat

---

## Related Documentation

- **Architecture Docs**: `docs/architecture/`
- **Deep Debt Reports**: Root directory (`DEEP_DEBT_*.md`)
- **Phase 2 Roadmap**: `PHASE2_ROADMAP_FEB05_2026.md`
- **Master Index**: `DEEP_DEBT_MASTER_INDEX.md`

---

**Document**: `docs/architecture/adrs/README.md`  
**Purpose**: Guide to Architecture Decision Records  
**Status**: ✅ Active  
**Next**: Write ADRs 002-004
