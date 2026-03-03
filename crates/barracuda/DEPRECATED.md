# DEPRECATED — This copy of barracuda is no longer maintained

**Date**: March 2, 2026
**Status**: Deprecated — use the standalone barraCuda primal instead

---

## What happened

The barracuda compute library has been extracted from toadStool into its own
standalone primal at `ecoPrimals/barraCuda/`. This embedded copy is kept for
reference but is no longer compiled as part of the toadStool workspace.

## Where to find the live code

```
ecoPrimals/barraCuda/crates/barracuda/    # standalone primal (active)
ecoPrimals/barraCuda/                     # full repository
```

GitHub: https://github.com/ecoPrimals/barraCuda

## How to rewire your dependency

In your `Cargo.toml`, change:

```toml
# Old (toadStool-embedded, deprecated):
barracuda = { path = "../../phase1/toadStool/crates/barracuda" }

# New (standalone barraCuda primal):
barracuda = { path = "../../barraCuda/crates/barracuda" }
```

Adjust the relative path based on your project's location within ecoPrimals.

## API compatibility

The standalone barraCuda has an identical API. hotSpring confirmed 716/716
tests pass with a single-line Cargo.toml path swap, no code changes needed.

## Timeline

- S88: Budding proposed
- S89: Extraction complete, all tests passing
- S89: hotSpring validated as first consumer
- S89: toadStool workspace deprecated this copy

The embedded copy will be removed in a future session after all consumers
have confirmed migration to the standalone primal.
