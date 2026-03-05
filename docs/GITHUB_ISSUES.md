# GitHub Issue Creation

This repository now includes an automation script to create the initial AuroraOS planning issues directly on GitHub.

## Prerequisites

- A GitHub personal access token with `repo` scope.
- Repository slug in `owner/repo` format.

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

