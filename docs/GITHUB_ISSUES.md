# GitHub Issue Creation

This repository now includes an automation script to create the initial AuroraOS planning issues directly on GitHub.

## Prerequisites

- GitHub CLI (`gh`) installed.
- A GitHub personal access token with `repo` scope (only needed for token-based auth fallback).
- Repository slug in `owner/repo` format.

## Authenticate GitHub CLI (`gh`)

Check current auth status:

```bash
gh auth status
```

### Recommended: device/browser flow

```bash
gh auth login -h github.com --web --git-protocol https
```

If prompted with a one-time code:
1. Open `https://github.com/login/device`
2. Enter the code shown by `gh`
3. Approve access

Verify:

```bash
gh auth status
```

### Fallback: personal access token

If device login fails (for example network/polling restrictions), use a token:

```bash
gh auth login -h github.com --with-token
```

Then paste a PAT with `repo` scope.

Optional check:

```bash
gh repo view owner/repo
```

## Command

```bash
GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo python3 scripts/github/create_issues.py
```

## Dry run

```bash
DRY_RUN=1 GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo python3 scripts/github/create_issues.py
```

## Output

On success, issue numbers and URLs are stored in:

- `docs/github_issues_created.json`
