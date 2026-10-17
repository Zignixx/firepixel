# Repository reset: one public initial commit

This page records how the private development history was replaced by a single English initial commit. The
branch `clean-main` holding that commit was prepared locally on 17 Oct 2026; **only the maintainer pushes.**
Delete this file in a later commit once the reset is done.

## 0. What is ready

- Branch **`clean-main`**: exactly one commit ("Firepixel: initial public snapshot"), built from branch `repo-en`
  (English docs and templates, merged with `main` at `ff9f391`, German working notes and generated images removed).
- **Backup of the full history:** `Desktop\Firepixel-Backup-2026-10-17_15-39\` — `firepixel-main.bundle` (git bundle
  of `main`) and `firepixel-worktree.zip` (working tree including German notes and screenshots). Check it before
  starting: `git bundle verify ..\..\Firepixel-Backup-2026-10-17_15-39\firepixel-main.bundle`.
- **German archive** of the removed paths (working notes, run reports, findings log, screenshots, `*_auto.md`;
  1345 files, list in `_archived_files.txt`): `C:\Users\Administrator\Desktop\REA\firepixel-archiv-2026-10-17\`.
  Exact list of removed paths: [`_archive_list.txt`](_archive_list.txt); see [ARCHIVE_NOTE.md](ARCHIVE_NOTE.md).
- Commits on `main` after `ff9f391` are **not** in `clean-main`; check `git log ff9f391..main` before step 1 and,
  if there are any, merge them into `repo-en` first and rebuild `clean-main` (or apply them on top afterwards).

## 1. Replace `main` (maintainer, PowerShell)

Run in the main checkout `C:\Users\Administrator\Desktop\REA\firepixel`, with a clean working tree
(`git status --short` empty). `clean-main` must not be checked out in another worktree (the preparing worktree
`fp-s5` was left detached).

```powershell
cd C:\Users\Administrator\Desktop\REA\firepixel
git status --short                 # must be empty
git checkout clean-main            # removes the archived German files from this checkout (they are in the archive)
git branch -D main                 # the old history stays in the backup bundle
git branch -m main                 # rename clean-main to main
git log --oneline | Measure-Object # -> 1 commit
git push --force origin main
```

`git branch -D main` fails while another worktree has `main` checked out; detach that worktree first
(`git -C ..\fp-sN checkout -q --detach`).

## 2. Re-point the worktrees

```powershell
git worktree prune
foreach ($w in 'fp-s2', 'fp-s3', 'fp-s4', 'fp-s5', 'fp-rel') {
    git -C ..\$w checkout -q --detach main
}
git worktree list
```

On 17 Oct 2026 further slots existed (`fp-s1`, `fp-s6` … `fp-s12`, all detached on old commits). Re-point them
the same way when they are used again, or remove them (`git worktree remove ..\fp-sN`).

Old local branches (`repo-en`, agent branches) still contain the private history: never push them; delete them
when they are no longer needed (`git branch -D <name>`). Delete stale remote branches and tags that point to the
old history (`git ls-remote origin`).

## 3. CI

CI stays **on demand only**: `windows-ci.yml` runs on `workflow_dispatch` and when called from
`prerelease.yml`; `macos-preview.yml` is manual as well. Nothing runs on push or pull request, so the force push
does not trigger builds.

## 4. After going public

- Repository settings: enable private vulnerability reporting (referenced by `SECURITY.md`), issues, and the
  issue templates under `.github/ISSUE_TEMPLATE`.
- Add the description and topics (rust, egui, graphics-editor, vector, bitmap, mcp).
- Attach screenshots to the first release instead of committing them.
