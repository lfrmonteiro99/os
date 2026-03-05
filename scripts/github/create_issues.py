#!/usr/bin/env python3
"""Create GitHub issues for AuroraOS roadmap via GitHub REST API.

Usage:
  GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo python3 scripts/github/create_issues.py

Optional:
  ISSUE_STATE_FILE=.issue_state.json  # defaults to docs/github_issues_created.json
  DRY_RUN=1                           # print payloads only
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

API = "https://api.github.com"
STATE_PATH = Path(os.getenv("ISSUE_STATE_FILE", "docs/github_issues_created.json"))
DRY_RUN = os.getenv("DRY_RUN", "0") == "1"

ISSUES = [
    ("M0-1: Create architecture decision record template", "Create ADR template and contribution guidance for architecture decisions."),
    ("M0-2: Draft ADR for kernel strategy selection", "Compare Linux-first vs custom-kernel path and record decision criteria."),
    ("M0-3: Draft ADR for compositor/render backend", "Capture rendering stack choice and fallback strategy."),
    ("M0-4: Draft ADR for IPC protocol format", "Define transport, serialization, and compatibility policy."),
    ("M0-5: Scaffold Rust workspace with lint/test defaults", "Initialize Cargo workspace layout and shared lint configuration."),
    ("M0-6: Set CI checks for rustfmt, clippy, cargo-deny, cargo-audit", "Add CI workflow to enforce baseline quality and supply-chain checks."),
    ("M1-1: Build VM image pipeline in CI", "Automate VM image creation and publish build artifacts."),
    ("M1-2: Prototype boot-to-shell handoff", "Implement minimal boot chain to shell stub with logs."),
    ("M1-3: Implement minimal service manager with demo services", "Support dependency ordering, restarts, and status."),
    ("M1-4: Structured logging crate and collector daemon", "Provide JSON logs and central aggregation API."),
    ("M1-5: Session manager prototype with login stub", "Create basic user session lifecycle and auth placeholder."),
    ("M1-6: Filesystem mount policy doc and spike", "Define mount layout and implement mount manager proof-of-concept."),
    ("M2-1: Design sandbox policy grammar", "Specify policy language for FS, network, and device permissions."),
    ("M2-2: Permission prompt UX prototype", "Build user-facing permission flow and persistence behavior."),
    ("M2-3: Compositor scene graph skeleton", "Implement node graph and basic render traversal."),
    ("M2-4: Blur and translucency shader prototype", "Prototype macOS-like blur/vibrancy effect pipeline."),
    ("M2-5: Input abstraction for keyboard/mouse/touchpad", "Normalize input events and gesture pre-processing."),
    ("M2-6: Window focus and stacking policy spec", "Document focus rules, z-order, and modality behavior."),
    ("M2-7: Dock prototype with app pinning", "Implement pinned apps and launch/activation behavior."),
    ("M2-8: Top bar status item framework", "Create extensible status area API with system indicators."),
    ("M2-9: Notification daemon and shell bridge", "Deliver daemon-driven notifications and shell rendering interface."),
    ("M3-1: SDK crate skeleton and hello-world template", "Create starter SDK crates and sample app."),
    ("M3-2: App manifest and entitlement schema v0", "Define app metadata, permissions, and validation schema."),
    ("M3-3: Package bundle format draft and signing sketch", "Specify app bundle layout and signature requirements."),
    ("M4-1: Files app navigation prototype", "Build initial file browser views and navigation model."),
    ("M4-2: Settings registry backend for Settings app", "Implement typed settings storage and reactive updates."),
    ("M4-3: Terminal app PTY integration prototype", "Connect terminal UI to PTY service with basic shell support."),
    ("M6-1: Update subsystem design doc (A/B vs snapshot)", "Evaluate update strategy and produce architecture doc."),
    ("M6-2: Threat model v1 workshop and document", "Run threat modeling and capture mitigations/priorities."),
    ("M7-1: Beta quality metrics dashboard definition", "Define launch metrics, SLOs, and reporting views."),
]


def request(method: str, url: str, token: str, payload: dict | None = None) -> dict:
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("X-GitHub-Api-Version", "2022-11-28")
    if payload is not None:
        req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=30) as res:
        return json.loads(res.read().decode("utf-8"))


def main() -> int:
    token = os.getenv("GITHUB_TOKEN")
    repo = os.getenv("GITHUB_REPOSITORY")

    if not token or not repo:
        print("ERROR: GITHUB_TOKEN and GITHUB_REPOSITORY=owner/repo are required.", file=sys.stderr)
        return 2

    owner, name = repo.split("/", 1)
    created = []

    for title, body in ISSUES:
        payload = {"title": title, "body": body, "labels": ["planning", "roadmap"]}
        if DRY_RUN:
            print(json.dumps(payload, indent=2))
            continue
        try:
            issue = request("POST", f"{API}/repos/{owner}/{name}/issues", token, payload)
            created.append({"title": title, "number": issue["number"], "url": issue["html_url"]})
            print(f"Created #{issue['number']}: {title}")
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf-8", errors="replace")
            print(f"Failed for '{title}': HTTP {e.code} {detail}", file=sys.stderr)
            return 1

    if not DRY_RUN:
        STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
        STATE_PATH.write_text(json.dumps(created, indent=2) + "\n", encoding="utf-8")
        print(f"Saved {len(created)} created issues to {STATE_PATH}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
