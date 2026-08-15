# shelfbox

## Overview

Keep AI context files, personal configs, and local secrets **visible in your editor** but **invisible to Git** — recoverable after reclones, worktrees, and index resets.

![shelfbox overview diagram](demo/shelfbox-overview.svg)

## Installation

For installer options, building from source, see [Installation in the Getting Started guide](docs/getting-started.md#installation).

### Linux and macOS

#### Homebrew

```sh
brew install masa-kjm/tap/shelfbox
```

#### Pre-built binary

The Unix installer uses `~/.local/bin` by default.

```sh
curl -fsSL https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.sh | sh
```

### Windows

> [!IMPORTANT]
> Before first use, Windows requires [additional setup](#required-setup).

#### Pre-built binary

The PowerShell installer uses `$Env:LOCALAPPDATA\Programs\shelfbox\bin` by default.

```powershell
irm https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.ps1 | iex
```

#### Required setup

Before using shelfbox on Windows, run:

```powershell
shelfbox config set mutation_durability best-effort
```

The default `require` mode depends on directory-level durability guarantees that Windows does not provide.

> [!NOTE]
> The default materialization strategy uses symlinks, which require Developer Mode or an elevated shell on Windows. Use [copy mode](docs/spec/copy-mode.md) when symlink creation is restricted.

## Quick Start

![shelfbox demo](demo/output/README.gif)

```sh
# Create a local-only file in your repository
echo "# Local instructions" > AGENTS.local.md

# Confirm it exists and is still untracked
git status --short

# Shelve it (keeps the path visible, stores canonical content outside Git)
shelfbox item add AGENTS.local.md

# Confirm Git no longer reports it
git status --short

# Show where shelfbox keeps its store
shelfbox config get store

# List repositories and managed items
shelfbox repo list --format plain
shelfbox item list --format plain
```

> [!NOTE]
> The demo focuses on the add and verification flow. Run `shelfbox item restore AGENTS.local.md` afterward to return to the original state.
> 
> Shelfbox manages only files that Git does not track. To make an already-tracked file local-only, follow [Make an Already-Tracked File Local Only](docs/workflows.md#make-an-already-tracked-file-local-only).

## Why Shelfbox

Some files need to stay in your repository tree so editors and tools can discover them, but they must never be committed. shelfbox separates those concerns: files remain visible at their original paths, canonical content is stored outside the repo, and Git stays out of the way.

| File | Why shelve it |
|---|---|
| `AGENTS.local.md`, `skills/my-skill/`, etc. | Personal AI assistant instructions |
| `notes/scratch.md` | Personal development notes |
| `config/local.yml` | Machine-specific config overrides |
| `.env` | Local secrets and credentials |

**Common workarounds have limits:**

- **`.gitignore`** — works for untracked files, but it is a shared repository policy. It is not suitable for one person's local paths unless the team chooses to commit that rule.
- **`.git/info/exclude`** — keeps ignore rules local, but only records paths. It does not preserve canonical content, track ownership, repair broken materializations, or help recover after a reclone or index loss.
- **`git update-index --skip-worktree`** — applies to tracked files, but its local index state does not survive a reclone, a new worktree, or index reset.
- **Manual move-and-symlink setups** — work initially, but leave recovery, ownership, and cleanup to the user.

Manual move-and-symlink setups are possible, but they do not provide lifecycle management. shelfbox adds **tracked ownership** and structured recovery: it materializes files at original paths, keeps them excluded via `.git/info/exclude`, and repairs broken materializations, lost repo associations after reclones, and orphaned store entries.

Canonical shelf data is stored under `<store>/repos/<repo-store-dir>/`: `manifest.json` keeps repository and item metadata, and `items/` keeps the actual file contents. See [Data model](docs/architecture/data-model.md) for details.

## More Features

- **Directory shelving** — shelve eligible files under a directory; each file remains an independent item: [`item add <PATH>`](docs/reference/item-commands.md#item-add-path)
- **Recovery after reclone** — re-associate a new clone with an existing shelf after restoring `repos/`: [`repo reclaim`](docs/reference/repo-commands.md#repo-reclaim)
- **Store recovery** — rebuild local cache files from canonical manifests: [`store rebuild-index`](docs/reference/store-commands.md#store-rebuild-index)
- **Copy mode** — leave an independent regular file instead of a symlink, useful when symlink creation is restricted: [Copy mode spec](docs/spec/copy-mode.md)

## Configuration

Optional config at `~/.config/shelfbox/config.toml` (respects `$XDG_CONFIG_HOME`):

```toml
# store = "/mnt/data/shelfbox-store"   # default: ~/.local/share/shelfbox
# default_format = "table"             # table | plain | json
# materialization = "symlink"          # symlink (default) | copy
# mutation_durability = "require"      # require (default) | best-effort
```

See [Config reference](docs/reference/config-commands.md) for all options and details.

## Non-goals

Shelfbox is a **single-machine** tool. Placing the store on external or network-synced storage is not officially supported — sync conflicts may leave items in an inconsistent state.

Multi-machine sync, secret encryption, and team-shared files are out of scope.

## Documentation

- [Getting Started](docs/getting-started.md) — installation, basic concepts, and first-time usage
- [Workflows](docs/workflows.md) — common tasks and recovery procedures  

See [docs/index.md](docs/index.md) for the full documentation set.

## License

MIT
