#!/usr/bin/env python3
"""
Lint PR titles + commit subjects for Conventional Commits.

This is intentionally lightweight (no Node/commitlint dependency).
It enforces the Conventional Commits *header* shape:
  type(scope)!: description

We lint:
- PR title: always
- commits in base..head (no merges): optional if base/head are provided
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from typing import Iterable, List, Optional, Tuple


ALLOWED_TYPES = (
    "build",
    "chore",
    "ci",
    "docs",
    "feat",
    "fix",
    "perf",
    "refactor",
    "revert",
    "style",
    "test",
)


HEADER_RE = re.compile(
    r"^(?P<type>"
    + "|".join(ALLOWED_TYPES)
    + r")"
    r"(?P<scope>\([^\)\r\n]+\))?"
    r"(?P<breaking>!)?"
    r": "
    r"(?P<desc>\S.+)$"
)


def run_git(args: List[str]) -> str:
    out = subprocess.check_output(["git", *args], stderr=subprocess.STDOUT)
    return out.decode("utf-8", errors="replace").strip()


def validate_header(s: str) -> Optional[str]:
    s = s.strip()
    if not s:
        return "empty"
    if not HEADER_RE.match(s):
        return (
            "does not match Conventional Commits header format "
            f"`type(scope)!: description` with type in {list(ALLOWED_TYPES)}"
        )
    return None


def iter_commit_subjects(base: str, head: str) -> Iterable[Tuple[str, str]]:
    """
    Yield (sha, subject) for commits in base..head.
    We intentionally skip merge commits to avoid false failures.
    """
    # %H\t%s gives us sha + subject in one line.
    raw = run_git(["log", "--no-merges", "--format=%H\t%s", f"{base}..{head}"])
    if not raw:
        return
    for line in raw.splitlines():
        sha, subject = line.split("\t", 1)
        yield sha, subject


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pr-title", default="", help="PR title to lint")
    ap.add_argument("--base", default="", help="Base SHA for commit range")
    ap.add_argument("--head", default="", help="Head SHA for commit range")
    args = ap.parse_args()

    errors: List[str] = []

    if args.pr_title:
        err = validate_header(args.pr_title)
        if err:
            errors.append(f"PR title: {err}\n  title: {args.pr_title}")

    if args.base and args.head:
        try:
            commits = list(iter_commit_subjects(args.base, args.head))
        except subprocess.CalledProcessError as e:
            errors.append(
                "Unable to read commit range via git log.\n"
                f"  range: {args.base}..{args.head}\n"
                f"  output: {e.output.decode('utf-8', errors='replace')}"
            )
            commits = []

        for sha, subject in commits:
            err = validate_header(subject)
            if err:
                errors.append(f"Commit {sha[:12]}: {err}\n  subject: {subject}")

    if errors:
        print("Conventional Commits lint failed:\n")
        for e in errors:
            print(f"- {e}\n")
        print("Examples:")
        print("  feat(api): add cursor pagination")
        print("  fix(auth)!: reject invalid session cookie")
        print("  docs: document local dev setup")
        return 1

    print("Conventional Commits lint passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

