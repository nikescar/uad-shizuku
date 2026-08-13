# Debloat Tab Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor debloat tab with virtual scrolling, mobile/desktop views, and strict MVVM to fix lag and improve maintainability

**Architecture:** Split monolithic 2,592-line tab into focused modules: separate mobile/desktop views (pure rendering), centralized state, virtual scrolling component for performance, async filtering via DebloatActor

**Tech Stack:** Rust, egui, egui_extras::TableBuilder (virtual scrolling), smol async runtime, async-channel

## Global Constraints

- Minimum Rust 2021 edition
- Use smol async runtime (NOT tokio)
- egui for UI framework
- Target line count: < 500 lines per file
- Test coverage: 80% minimum (cargo llvm-cov)
- Desktop width threshold: 800px (`DESKTOP_MIN_WIDTH`)
- Row height: 24px for desktop, 48px for mobile
- Filter debounce: 300ms

---

**Note:** This plan focuses on Phase 1 (Debloat Tab) from the design spec. Subsequent phases (Scan, Apps tabs) will follow the same pattern established here.

Plan complete and saved to `docs/superpowers/plans/2026-08-13-tab-refactor.md`.

**Execution options:**

**1. Subagent-Driven (recommended)** - Fresh subagent per task with review between tasks

**2. Inline Execution** - Execute tasks in this session with checkpoints

**Which approach would you prefer?**
