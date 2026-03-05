# AuroraOS (Rust) — Full Delivery Plan

## 0) Product framing

**Vision:** Build a modern desktop operating system with polished visuals and interaction quality inspired by macOS, implemented primarily in Rust, while respecting legal boundaries (inspiration, not cloning).

**Design principles:**
- Native-feeling motion, typography, translucency, and spacing.
- Security-first defaults (sandboxing, signed packages, privilege separation).
- Reliable updates and recoverability.
- Hardware abstraction and portability where practical.
- Strong developer ergonomics for apps and system services.

**Non-goals (first release):**
- Full binary compatibility with macOS.
- Broad legacy hardware support.
- Running all Linux desktop apps unmodified.

---

## 1) Architecture decisions (must be locked early)

### Kernel strategy
- **Option A (recommended for speed):** Rust-first user space on Linux kernel initially.
- **Option B (long-term):** Custom microkernel/hybrid kernel in Rust.

**Recommendation:** Start with Option A to ship user-visible value quickly, then progressively replace components and/or pivot kernel once product/UX is stable.

### Graphics stack
- Display server/compositor based on Wayland concepts; custom shell compositor in Rust.
- GPU APIs: Vulkan/Metal abstraction (via wgpu where useful).
- Renderer pipeline: scene graph, effects pipeline (blur, vibrancy), text shaping (HarfBuzz equivalent bindings).

### System component model
- Service manager (system + user units), strongly sandboxed daemons.
- Capability-based IPC.
- Immutable-ish system image + atomic updates.

### Rust tech baseline
- Rust stable toolchain + edition 2021/2024.
- Core crates likely: `winit`/`smithay`, `wgpu`, `serde`, `tokio`, `tracing`, `thiserror`, `anyhow`.

---

## 2) Program milestones

## Milestone M0 — Foundations & Feasibility (4–8 weeks)
**Goal:** De-risk architecture and prove core UX primitives.

### Deliverables
- Architecture RFCs approved (kernel strategy, graphics pipeline, security model).
- Bootable prototype (VM) to desktop shell stub.
- Design language v1 (colors, typography, layout metrics, motion curves).
- CI/CD baseline (build/test/lint/artifact pipeline).

### Exit criteria
- Prototype boots to shell in <15s (VM baseline).
- Window rendering, input handling, and composited blur demo functional.
- Team development loop documented and reproducible.

### Issues
1. **Issue M0-1:** Create system architecture RFC pack.
2. **Issue M0-2:** Establish Rust workspace and crate boundaries.
3. **Issue M0-3:** Build boot flow prototype (UEFI/bootloader -> init -> shell).
4. **Issue M0-4:** Implement compositor POC with blur/vibrancy effect.
5. **Issue M0-5:** Define HIG-inspired design tokens and motion specs.
6. **Issue M0-6:** Set up CI (fmt, clippy, tests, security audit).

---

## Milestone M1 — Core OS Runtime (8–12 weeks)
**Goal:** Stable base system services and secure process model.

### Deliverables
- Init/service manager.
- Process supervision and logging.
- Filesystem layout + mount manager.
- User/session model (multi-user capable, single-user optimized).
- Permissions and sandbox policy engine.

### Exit criteria
- Boot to login/session reliably across reboots.
- Crash recovery for core services.
- Basic privilege separation validated by threat-model checks.

### Issues
1. **Issue M1-1:** Implement service manager with dependency graph.
2. **Issue M1-2:** Journaling/log pipeline with structured logs.
3. **Issue M1-3:** Session and user manager daemon.
4. **Issue M1-4:** Sandbox policy v1 (filesystem/network/device scopes).
5. **Issue M1-5:** Secure secret storage API.
6. **Issue M1-6:** System configuration schema + validator.

---

## Milestone M2 — Graphics, Windowing, and Desktop Shell (10–16 weeks)
**Goal:** Deliver a polished, macOS-like visual desktop experience.

### Deliverables
- Compositor with animations, blur, shadows, transparency.
- Window manager (tiling+floating hybrid with desktop defaults).
- Desktop shell: top bar, dock, app switcher, notifications, control center.
- Input stack (keyboard, mouse, touchpad gestures).
- Font rendering, icon system, and accessibility hooks.

### Exit criteria
- Smooth interaction targets (e.g., 60fps on baseline hardware).
- Multi-window workflows stable.
- Gesture navigation consistent and intuitive.

### Issues
1. **Issue M2-1:** Scene graph renderer and damage tracking.
2. **Issue M2-2:** Window lifecycle protocol and focus model.
3. **Issue M2-3:** Dock component + app launching model.
4. **Issue M2-4:** Top menu/status bar architecture.
5. **Issue M2-5:** Notification center + quick settings panel.
6. **Issue M2-6:** Gesture recognizer pipeline and bindings.
7. **Issue M2-7:** Accessibility primitives (focus rings, screen-reader events).

---

## Milestone M3 — Developer Platform & App Runtime (8–14 weeks)
**Goal:** Enable third-party applications safely.

### Deliverables
- App SDK (Rust-first), UI toolkit, and packaging format.
- App sandboxing and permissions prompts.
- App lifecycle APIs (open/save panels, notifications, background tasks).
- Dev tools: CLI scaffolding, debugger integration, profiler hooks.

### Exit criteria
- At least 3 sample apps shipped (Notes, Terminal, Files).
- App installation/update/remove is reliable and reversible.
- SDK docs + templates available.

### Issues
1. **Issue M3-1:** Define app manifest and entitlement model.
2. **Issue M3-2:** Build package manager and signed bundle format.
3. **Issue M3-3:** Create UI toolkit primitives (views, layout, animation).
4. **Issue M3-4:** Implement permission broker service.
5. **Issue M3-5:** CLI scaffolder and sample app templates.
6. **Issue M3-6:** SDK docs site and API reference generation.

---

## Milestone M4 — System Applications & UX Completeness (8–12 weeks)
**Goal:** Ship daily-driver desktop essentials.

### Deliverables
- Finder-like file manager.
- System settings app.
- Terminal emulator.
- Browser integration strategy (embedded or packaged browser).
- Media, screenshot, clipboard history, and search launcher.

### Exit criteria
- Daily workflows complete without terminal dependence.
- Visual and interaction consistency across built-in apps.

### Issues
1. **Issue M4-1:** Files app with metadata, preview, drag/drop.
2. **Issue M4-2:** Settings app with typed configuration backends.
3. **Issue M4-3:** Terminal app + PTY service integration.
4. **Issue M4-4:** Global search/indexing daemon.
5. **Issue M4-5:** Screenshot and screen recording utility.
6. **Issue M4-6:** Clipboard manager and universal actions.

---

## Milestone M5 — Hardware, Networking, and Power (10–16 weeks)
**Goal:** Improve hardware compatibility and laptop usability.

### Deliverables
- Network manager (Wi-Fi, Ethernet, VPN baseline).
- Audio stack integration.
- Power management (sleep/resume/battery profiles).
- Display management (multi-monitor, scaling, color profiles).
- Device support matrix and telemetry (opt-in).

### Exit criteria
- Stable laptop session with suspend/resume.
- Multi-display setup usable for productivity.
- Network reconnection robust under mobility.

### Issues
1. **Issue M5-1:** Network daemon + UI integration.
2. **Issue M5-2:** Audio routing and per-app volume controls.
3. **Issue M5-3:** Power profile service and idle policies.
4. **Issue M5-4:** Multi-display compositor logic and settings.
5. **Issue M5-5:** Hardware compatibility lab automation.
6. **Issue M5-6:** Driver abstraction and fallback behavior docs.

---

## Milestone M6 — Security Hardening & Update System (6–10 weeks)
**Goal:** Production-grade trust and recoverability.

### Deliverables
- Secure boot chain strategy.
- Signed system images and package signatures.
- Atomic OTA updates with rollback.
- Incident/diagnostics mode and recovery partition.
- Baseline security audits and fuzzing harness.

### Exit criteria
- Update can be interrupted without bricking.
- Security review findings triaged and major issues fixed.

### Issues
1. **Issue M6-1:** Implement artifact signing pipeline.
2. **Issue M6-2:** A/B or snapshot-based update mechanism.
3. **Issue M6-3:** Recovery boot flow and diagnostics shell.
4. **Issue M6-4:** Threat model refresh and hardening checklist.
5. **Issue M6-5:** Fuzz IPC, parsers, and privilege boundaries.
6. **Issue M6-6:** Security policy compliance automation.

---

## Milestone M7 — Beta Program & Launch Readiness (6–12 weeks)
**Goal:** Prepare public beta and release candidate.

### Deliverables
- Installer/ISO images and onboarding flow.
- Crash reporting and performance telemetry (opt-in).
- Localization baseline.
- Release notes + migration guides.
- Support tooling and issue triage runbooks.

### Exit criteria
- Beta quality gates met (crash rates, boot success, UX defects).
- Release candidate signed and reproducible.

### Issues
1. **Issue M7-1:** Build installer UX and partitioning flow.
2. **Issue M7-2:** Telemetry pipeline + privacy controls.
3. **Issue M7-3:** Localization extraction and translation infra.
4. **Issue M7-4:** End-to-end upgrade test matrix.
5. **Issue M7-5:** Beta feedback tooling and triage SOP.
6. **Issue M7-6:** RC sign-off checklist and launch war room playbook.

---

## 3) Cross-cutting tracks (run continuously)

### A) Design system
- Token versioning, animation language, component library.
- Accessibility standards (contrast, keyboard-only nav, screen reader).

### B) Performance engineering
- Frame pacing, startup time budgets, memory pressure handling.
- Benchmarks in CI for regression detection.

### C) Reliability engineering
- Chaos testing of service crashes and restart policies.
- Failsafe defaults and safe mode paths.

### D) Documentation & DX
- Architecture docs, runbooks, troubleshooting trees.
- Contributor onboarding and coding conventions.

### E) Governance & legal
- Trademark-safe branding.
- OSS license compliance and third-party notices.

---

## 4) Initial repository/workspace layout (Rust)

```text
auroraos/
  Cargo.toml
  rust-toolchain.toml
  crates/
    boot/
    init/
    svc-manager/
    ipc/
    security-policy/
    compositor/
    wm/
    shell/
    sdk/
    pkg/
    updater/
    apps/
      files/
      settings/
      terminal/
  docs/
    architecture/
    threat-model/
    roadmap/
  assets/
    icons/
    fonts/
    design-tokens/
```

---

## 5) Team composition (suggested)

- 1 Technical Program Manager
- 2 Kernel/low-level engineers
- 3 Platform/runtime engineers
- 3 Graphics/UI engineers
- 2 Security engineers
- 2 App/SDK engineers
- 1 QA automation engineer
- 1 Product designer
- 1 Developer relations/documentation

---

## 6) Risk register (top risks + mitigation)

1. **Graphics complexity risk** — Mitigate by proving compositor path in M0 and locking rendering constraints early.
2. **Driver/hardware fragmentation risk** — Start with constrained hardware target matrix.
3. **Security model drift risk** — Gate milestones with threat-model reviews.
4. **Scope creep risk** — Enforce strict non-goals and freeze lists each milestone.
5. **Performance regressions risk** — Add benchmark gates before M2 completion.
6. **App ecosystem cold-start risk** — Invest early in SDK, templates, and high-quality sample apps.

---

## 7) Suggested issue tracker taxonomy

- **Epic:** milestone-level outcome.
- **Feature:** major subsystem increment.
- **Task:** implementation unit (1–3 days).
- **Bug:** defect with severity labels.
- **Spike:** time-boxed investigation.

**Labels:** `area:*`, `milestone:*`, `priority:*`, `risk:*`, `security`, `perf`, `ux`, `blocked`.

---

## 8) First 30 concrete issues to open now

1. Create architecture decision record template.
2. Draft ADR: kernel strategy selection.
3. Draft ADR: compositor/render backend.
4. Draft ADR: IPC protocol format.
5. Scaffold Rust workspace with lint/test defaults.
6. Set `clippy`, `rustfmt`, `cargo-deny`, `cargo-audit` CI checks.
7. Build VM image pipeline in CI.
8. Prototype boot-to-shell handoff.
9. Implement minimal service manager with 3 demo services.
10. Structured logging crate + collector daemon.
11. Session manager prototype with login stub.
12. Filesystem mount policy document + implementation spike.
13. Sandbox policy grammar design.
14. Permission prompt UX prototype.
15. Compositor scene graph skeleton.
16. Blur/translucency shader prototype.
17. Input abstraction layer for keyboard/mouse/touchpad.
18. Window focus/stacking policy spec.
19. Dock prototype with app pinning.
20. Top bar status item framework.
21. Notification daemon + UI shell bridge.
22. SDK crate skeleton and hello-world app template.
23. App manifest + entitlement schema v0.
24. Package bundle format draft + signing sketch.
25. Files app navigation prototype.
26. Settings app settings registry backend.
27. Terminal emulator PTY integration prototype.
28. Update subsystem design doc (A/B vs snapshot).
29. Threat model v1 workshop and document.
30. Beta quality metrics definition dashboard.

---

## 9) Release quality gates (must-pass)

- Boot success rate >= 99% on supported hardware matrix.
- No known critical privilege-escalation bugs.
- 95th percentile app launch under target budget.
- Compositor frame drops under agreed threshold.
- Update rollback validated across power-loss scenarios.
- Installer success and recovery flow validated.

---

## 10) Suggested cadence

- 2-week sprints.
- Weekly architecture review.
- Weekly security + reliability review.
- Milestone demo at sprint end.
- Milestone freeze 1 week before cut.

