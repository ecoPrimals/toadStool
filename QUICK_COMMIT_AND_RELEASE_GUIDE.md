# Quick Commit & Release Guide

> **TL;DR**: Commit often, clean branches regularly, release binaries via GitHub Releases (never commit large binaries to repo!)

---

## 📋 Table of Contents

1. [Commit & Push Changes](#-commit--push-changes)
2. [Clean Old Branches](#-clean-old-branches)
3. [Release Binaries](#-release-binaries-dont-push-to-repo)
4. [Why This Matters](#-why-this-matters)
5. [Common Workflows](#-common-workflows)
6. [Troubleshooting](#-troubleshooting)

---

## ✅ Commit & Push Changes

### Basic Workflow

```bash
# Stage all changes
git add -A

# Commit with message
git commit -m "feat: Your descriptive message"

# Push to current branch
git push origin $(git branch --show-current)
```

### Commit Message Conventions

We use conventional commits for clarity:

```bash
# Features
git commit -m "feat: Add capability discovery system"

# Bug fixes
git commit -m "fix: Resolve unwrap in hot path"

# Documentation
git commit -m "docs: Update API documentation"

# Chores (build, dependencies, cleanup)
git commit -m "chore: Update dependencies"

# Refactoring
git commit -m "refactor: Simplify error handling"

# Tests
git commit -m "test: Add integration tests for discovery"

# Performance
git commit -m "perf: Optimize clone operations"
```

### Quick Check Before Commit

```bash
# Check what's changed
git status

# Review changes
git diff

# Run tests
cargo test --workspace

# Check lints
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all
```

---

## 🧹 Clean Old Branches

### Safe Branch Cleanup

```bash
# Update remote references (removes deleted remote branches)
git fetch -p

# Delete local branches that are merged into main
git branch --merged main | grep -v "main" | grep -v "\*" | xargs -r git branch -d

# List all local branches
git branch -a

# List branches with last commit date (helpful!)
git for-each-ref --sort=-committerdate refs/heads/ --format='%(committerdate:short) %(refname:short) %(subject)'
```

### Delete Specific Branches

```bash
# Delete local branch (safe - won't delete if unmerged)
git branch -d old-branch-name

# Force delete local branch (careful!)
git branch -D old-branch-name

# Delete remote branch
git push origin --delete old-branch-name
```

### Our Branch Strategy

- `main` / `master` - Production-ready code
- `polish/*` - Polish and cleanup branches
- `feature/*` - New features
- `fix/*` - Bug fixes
- `refactor/*` - Code improvements

**Cleanup Schedule**: After PR merge, delete feature branches immediately.

---

## 📦 Release Binaries (Don't Push to Repo!)

### Why Not Commit Binaries?

❌ **NEVER DO THIS**:
```bash
git add target/release/toadstool-cli  # ← DON'T!
```

✅ **DO THIS INSTEAD**: Use GitHub Releases

### The Right Way: GitHub Releases

#### Step 1: Build Release Binary

```bash
# Build all ToadStool binaries
cargo build --release --workspace

# Or specific binary
cargo build --release -p toadstool-cli

# Binaries are in: target/release/
ls -lh target/release/toadstool-*
```

#### Step 2: Create Checksums

```bash
# Create checksums for verification
cd target/release/

# Individual checksums
sha256sum toadstool-cli > toadstool-cli.sha256
sha256sum toadstool-executor > toadstool-executor.sha256

# Or all at once
sha256sum toadstool-* > checksums.sha256

cd ../..
```

#### Step 3: Create Git Tag

```bash
# Create annotated tag (recommended)
git tag -a v0.1.0-integration -m "Integration checkpoint - Dec 23, 2025"

# Push tag to remote
git push origin v0.1.0-integration

# Verify tag
git tag -l -n1 v0.1.0-integration
```

#### Step 4: Install GitHub CLI (One-time Setup)

```bash
# Install gh CLI
sudo apt install gh

# Or via snap
sudo snap install gh

# Authenticate
gh auth login --web

# Verify
gh auth status
```

#### Step 5: Create Release with Binaries

```bash
# Single binary
gh release create v0.1.0-integration \
  target/release/toadstool-cli \
  target/release/toadstool-cli.sha256 \
  --title "ToadStool v0.1.0 - Integration Checkpoint" \
  --notes "Ready for testing. See CHANGELOG.md for details." \
  --prerelease

# Multiple binaries
gh release create v0.1.0-integration \
  target/release/toadstool-cli \
  target/release/toadstool-executor \
  target/release/checksums.sha256 \
  --title "ToadStool v0.1.0 - Integration Checkpoint" \
  --notes-file RELEASE_NOTES.md \
  --prerelease

# Stable release (not prerelease)
gh release create v1.0.0 \
  target/release/toadstool-cli \
  target/release/checksums.sha256 \
  --title "ToadStool v1.0.0 - Stable Release" \
  --notes-file RELEASE_NOTES.md \
  --latest
```

#### Step 6: Share Download URL

Release URL format:
```
https://github.com/ecoPrimals/toadStool/releases/tag/v0.1.0-integration
```

Direct download:
```bash
# Download binary
wget https://github.com/ecoPrimals/toadStool/releases/download/v0.1.0-integration/toadstool-cli

# Download checksum
wget https://github.com/ecoPrimals/toadStool/releases/download/v0.1.0-integration/toadstool-cli.sha256

# Verify
sha256sum -c toadstool-cli.sha256

# Make executable
chmod +x toadstool-cli

# Run
./toadstool-cli --version
```

---

## 🎯 Why This Matters

### Benefits of This Workflow

| Practice | Why It Matters |
|----------|---------------|
| **Conventional Commits** | Easy to generate changelogs, understand history |
| **Small, Frequent Commits** | Easier to review, revert if needed |
| **Clean Branches** | Faster navigation, clearer history |
| **Binaries in Releases** | Git stays fast, no bloat, proper versioning |
| **Tags + Releases** | Traceable checkpoints, easy rollbacks |
| **Checksums** | Verify binary integrity, security |

### What Happens If You Commit Binaries?

❌ **Problems**:
- Git repo becomes **huge** (100MB+ per binary × history)
- Clone times slow to a crawl
- GitHub may **reject pushes** (>100MB files)
- Impossible to remove without `git filter-branch` (painful!)
- Wastes everyone's bandwidth

✅ **Solution**: GitHub Releases
- Binaries stored separately from Git history
- Download on-demand only
- No repo size impact
- Easy to replace/update

---

## 🔄 Common Workflows

### Daily Development

```bash
# 1. Start your day
git pull origin main
git checkout -b feature/my-awesome-feature

# 2. Make changes, commit often
git add src/my_changes.rs
git commit -m "feat: Add initial implementation"

# ... more changes ...
git add tests/
git commit -m "test: Add tests for new feature"

# 3. Push to remote
git push origin feature/my-awesome-feature

# 4. Create PR on GitHub
gh pr create --title "Add awesome feature" --body "Description here"
```

### Pre-Release Checklist

```bash
# 1. Ensure main is clean
git checkout main
git pull origin main

# 2. Run full test suite
cargo test --workspace --release
cargo clippy --workspace -- -D warnings
cargo fmt --all --check

# 3. Update version in Cargo.toml files
# (Use workspace version or per-crate versions)

# 4. Update CHANGELOG.md
# Document all changes since last release

# 5. Commit version bump
git add .
git commit -m "chore: Bump version to v0.1.0"
git push origin main

# 6. Create release (see Release Binaries section)
```

### Hotfix Workflow

```bash
# 1. Branch from main
git checkout main
git pull origin main
git checkout -b fix/critical-bug

# 2. Fix the issue
# ... make changes ...
git add .
git commit -m "fix: Resolve critical security issue"

# 3. Push and PR immediately
git push origin fix/critical-bug
gh pr create --title "HOTFIX: Critical security issue" --body "Details..."

# 4. After merge, create patch release
git checkout main
git pull origin main
git tag -a v0.1.1 -m "Hotfix: Security patch"
git push origin v0.1.1

# 5. Build and release
cargo build --release
gh release create v0.1.1 \
  target/release/toadstool-cli \
  --title "ToadStool v0.1.1 - Security Patch" \
  --notes "Critical security fix. Update immediately." \
  --latest
```

---

## 🔧 Troubleshooting

### "Push rejected - large file detected"

```bash
# 1. Don't panic! Don't force push!

# 2. Check what's large
git rev-list --objects --all | \
  git cat-file --batch-check='%(objectsize:disk) %(objectname) %(rest)' | \
  sort -rn | head -20

# 3. Remove from staging
git reset HEAD path/to/large/file

# 4. Add to .gitignore
echo "path/to/large/file" >> .gitignore
echo "*.tar.gz" >> .gitignore

# 5. Commit gitignore
git add .gitignore
git commit -m "chore: Update gitignore for large files"

# 6. If already committed, see "Remove from history" below
```

### Remove File from Git History

⚠️ **WARNING**: This rewrites history! Coordinate with team first.

```bash
# Method 1: filter-branch (built-in)
git filter-branch --force --index-filter \
  'git rm --cached --ignore-unmatch path/to/large/file' \
  --prune-empty --tag-name-filter cat -- --all

# Clean up
rm -rf .git/refs/original/
git reflog expire --expire=now --all
git gc --prune=now --aggressive

# Force push (coordinate with team!)
git push origin --force --all
```

### "Branch has diverged"

```bash
# Option 1: Merge (preserves history)
git pull origin main
git push origin your-branch

# Option 2: Rebase (cleaner history)
git pull --rebase origin main
git push origin your-branch --force-with-lease

# Option 3: Reset (if you want to discard local commits)
git fetch origin
git reset --hard origin/your-branch
```

### "Cannot delete branch - not fully merged"

```bash
# Check if actually merged
git branch --merged main | grep your-branch

# If it's safe to delete
git branch -D your-branch  # Force delete

# If you're unsure, create backup first
git branch backup-your-branch your-branch
git branch -D your-branch
```

### "Tag already exists"

```bash
# Delete local tag
git tag -d v0.1.0

# Delete remote tag
git push origin --delete v0.1.0

# Create new tag
git tag -a v0.1.0 -m "Updated tag"
git push origin v0.1.0
```

---

## 📚 References

### ToadStool Specific

- **Main Repository**: https://github.com/ecoPrimals/toadStool
- **Example Release**: https://github.com/ecoPrimals/bearDog/releases/tag/v0.9.0-integration-dec23
- **Status**: See `STATUS.md` for current project status
- **Architecture**: See `specs/UNIVERSAL_COMPUTE_PLATFORM.md`

### Learning Resources

- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitHub CLI Manual](https://cli.github.com/manual/)
- [Git Book](https://git-scm.com/book/en/v2)
- [Semantic Versioning](https://semver.org/)

### Tools

```bash
# Helpful aliases (add to ~/.gitconfig)
[alias]
    co = checkout
    br = branch
    ci = commit
    st = status
    lg = log --graph --oneline --all --decorate
    last = log -1 HEAD
    unstage = reset HEAD --
    branches = for-each-ref --sort=-committerdate refs/heads/ --format='%(committerdate:short) %(refname:short) %(subject)'
```

---

## 🤝 Team Collaboration

### Before You Push

✅ **Do**:
- Run tests: `cargo test --workspace`
- Run lints: `cargo clippy --workspace -- -D warnings`
- Format code: `cargo fmt --all`
- Write clear commit messages
- Push to feature branch first

❌ **Don't**:
- Push directly to `main` (unless hotfix)
- Commit binaries or large files
- Force push shared branches without coordinating
- Commit secrets or credentials
- Use `unwrap()` in production code

### Code Review Process

1. Create feature branch
2. Make changes with clear commits
3. Push and create PR
4. Request review from team
5. Address feedback
6. Merge after approval
7. Delete feature branch

---

## 🎯 Quick Reference Card

### Everyday Commands

```bash
# Status
git status
git log --oneline -5

# Commit
git add -A
git commit -m "feat: Your message"
git push origin $(git branch --show-current)

# Branch
git checkout -b feature/new-thing
git branch -d old-branch

# Update
git fetch -p
git pull origin main

# Release
cargo build --release
git tag -a v0.1.0 -m "Release"
git push origin v0.1.0
gh release create v0.1.0 target/release/toadstool-cli
```

---

## 📞 Need Help?

- **Team Chat**: Ask in ecoPrimals team channel
- **Issues**: https://github.com/ecoPrimals/toadStool/issues
- **Documentation**: See `START_HERE.md`

---

**🐻 ecoPrimals - Keep it clean, keep it sovereign!**

*Last Updated: December 23, 2025*

