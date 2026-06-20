# 5-Layer Cognitive Architecture — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-architect trit-core into a 5-layer cognitive program body with steady anchors, scenario-driven hook manager, 10 dynamic adapter modules, ternary decision engine facade, and closed-loop feedback.

**Architecture:** Path 2 — module-internal layering. New directories `src/anchor/`, `src/hook/`, `src/adapters/`, `src/feedback/` are added alongside existing `src/core/` and `src/meta/`. Existing `src/attention/`, `src/knowledge/`, `src/reflexive/` are migrated into `src/adapters/`. Layer communication is through typed Rust traits and structs; no cross-layer field access.

**Tech Stack:** Rust 2021 edition, serde, thiserror, tracing, chrono (existing deps). No new dependencies.

**Implementation order:** Phase 1 (scaffold) → Phase 2 (Layer 2 Hook Manager) → Phase 3 (Layer 3 Adapters) → Phase 4 (Layer 1 Anchors) → Phase 5 (Layer 4 Facade) → Phase 6 (Layer 5 Feedback) → Phase 7 (Integration).

---
