#!/usr/bin/env bash
# Copy to scripts/init-env.sh in each adopting repository and commit it.
set -euo pipefail
standards=${ES_STANDARDS_DIR:-"$HOME/.local/share/unibitorg/engineering-standards"}
repository=${ES_STANDARDS_REPO:-https://github.com/unibitorg/engineering-standards.git}
branch=${ES_STANDARDS_BRANCH:-main}
update=0
case "${1:-}" in
  --update) update=1; shift ;;
  --help) echo "Usage: $0 [--update]"; exit 0 ;;
esac
[ "$#" -eq 0 ] || { echo "Unknown argument: $*" >&2; exit 2; }
case "$standards" in
  /*) ;;
  *) echo "ES_STANDARDS_DIR must be an absolute, permanent path" >&2; exit 1 ;;
esac

# Diagnose the caller's repository before changing directories or taking the fast path.
caller=$(cd "$(dirname "$0")" && pwd -P)
local_hooks=$(git -C "$caller" config --local --includes --get core.hooksPath 2>/dev/null || true)
worktree_hooks=$(git -C "$caller" config --worktree --includes --get core.hooksPath 2>/dev/null || true)
if [ -n "$local_hooks$worktree_hooks" ]; then
  hook_dir=$(git -C "$caller" rev-parse --path-format=absolute --git-path hooks)
  echo "Repository-local core.hooksPath overrides organization hooks: $hook_dir" >&2
  for stage in pre-commit commit-msg pre-push; do
    echo "Integrate organization checks at the start of: $hook_dir/$stage" >&2
  done
  echo "Copy the header from: $standards/tools/hooks/local-hook.template.sh" >&2
  echo "Keep the existing hook body; the header preserves arguments and pre-push stdin and rejects failures." >&2
fi

# Compare spelling variants without changing the URL used by Git's credential helper.
normalize_repository() {
  local url=${1%/}
  printf '%s\n' "${url%.git}"
}

if [ -e "$standards" ]; then
  [ -d "$standards/.git" ] || { echo "Not a standalone checkout: $standards" >&2; exit 1; }
  if [ -n "$(git --no-optional-locks -C "$standards" status --porcelain --untracked-files=normal)" ]; then
    echo "Local changes in $standards; checkout left untouched. Review and commit them, then rerun $0." >&2
    exit 1
  fi
  if [ "$(normalize_repository "$(git -C "$standards" remote get-url origin)")" != "$(normalize_repository "$repository")" ] ||
     [ "$(git -C "$standards" branch --show-current)" != "$branch" ]; then
    echo "Expected origin $repository and branch $branch in $standards; checkout left untouched." >&2
    exit 1
  fi
  if [ "$update" = 0 ] && bash "$standards/tools/hooks/install.sh" --check >/dev/null 2>&1; then
    echo "Organization hooks already installed; no changes (use --update to fetch and fast-forward)."
    exit 0
  fi
  git -C "$standards" fetch --no-tags origin "$branch"
  git -C "$standards" merge --ff-only FETCH_HEAD
else
  if ! mkdir -p "$(dirname "$standards")" || [ ! -w "$(dirname "$standards")" ]; then
    echo "Cannot write the organization hooks directory: $(dirname "$standards")" >&2
    echo "The default requires a writable HOME/.local directory (shared accounts may forbid this)." >&2
    echo "Set ES_STANDARDS_DIR to a writable, permanent absolute path, for example under XDG_DATA_HOME, then rerun $0." >&2
    exit 1
  fi
  git clone --depth 1 --single-branch --branch "$branch" "$repository" "$standards"
fi
if ! bash "$standards/tools/hooks/install.sh"; then
  echo "Installation failed. For a global core.hooksPath conflict, review and retain the existing hooks first." >&2
  echo "Only if replacement is intended, run: bash \"$standards/tools/hooks/install.sh\" --force" >&2
  echo "Otherwise integrate the dispatcher into the existing hooks; see $standards/tools/hooks/README.md." >&2
  exit 1
fi
