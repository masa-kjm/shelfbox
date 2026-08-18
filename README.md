# shelfbox

## Overview

Keep local files—notes, AI context, and personal configs—**visible in your editor** but **out of Git**. Store them portably across repositories, worktrees, and reclones.

![shelfbox overview diagram](demo/shelfbox-overview.svg)

## Why Shelfbox

Local AI context files, notes, scripts, and machine-specific files often need to live inside a repository tree so editors and tools can discover them—but they shouldn't be committed.

**Common workarounds have limits:**

- **`.gitignore`** — works for untracked files, but it is shared repository policy rather than a personal local-file mechanism.
- **`.git/info/exclude`** — keeps ignore rules local, but only records paths. It does not preserve the files themselves or help restore them after a reclone.
- **Manual move-and-symlink setups** — keep files outside the repository, but leave ownership, recovery, and cleanup to the user.

Shelfbox manages that lifecycle explicitly: it stores canonical content outside the repository, materializes files at their original paths, keeps them excluded from Git, and can repair or recover that local context across worktrees and after recloning.

## Quick Start

![shelfbox demo](demo/output/README.gif)

```sh
# Create a local-only file in your repository
echo "# Local instructions" > AGENTS.local.md

# Confirm it exists and is still untracked
git status --short

# Shelve it (keeps the path visible, stores canonical content outside Git)
shelfbox item add AGENTS.local.md

# The file remains available at its original path
cat AGENTS.local.md

# Confirm Git no longer reports it
git status --short

# Confirm shelfbox is managing it
shelfbox item list --format plain
```

For an annotated first workflow, see [Start Managing a Local File](docs/guide/workflows.md#start-managing-a-local-file).

## Installation

For source builds and installation script options, see the [installation guide](docs/guide/installation.md).  
To remove shelfbox and optionally clean up its local data, see [Uninstallation](docs/guide/installation.md#uninstallation).

### Linux and macOS

#### Homebrew

```sh
brew install masa-kjm/tap/shelfbox
```

#### [Pre-built binary](https://github.com/masa-kjm/shelfbox/releases)

The Unix installer uses `~/.local/bin` by default.

```sh
curl -fsSL https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.sh | sh
```

### Windows

#### [Pre-built binary](https://github.com/masa-kjm/shelfbox/releases)

The PowerShell installer uses `$Env:LOCALAPPDATA\Programs\shelfbox\bin` by default.

```powershell
irm https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.ps1 | iex
```

> [!IMPORTANT]
> Windows setup: Before using shelfbox, enable best-effort durability:
> 
> ```powershell
> shelfbox config set mutation_durability best-effort
> ```
> 
> Windows does not provide the directory-level durability guarantees required by the default require mode.
> Symlink materialization also requires Developer Mode or an elevated shell. If symlinks are unavailable, use [copy mode](docs/spec/copy-mode.md).

## More Features

- **Recovery after reclone** — re-associate a new clone with an existing shelf after restoring `repos/`: [`repo reclaim`](docs/reference/repo-commands.md#repo-reclaim)
- **Directory shelving** — shelve eligible files under a directory; each file remains an independent item: [`item add <PATH>`](docs/reference/item-commands.md#item-add-path)
- **Copy mode** — leave an independent regular file instead of a symlink, useful when symlink creation is restricted: [Copy mode spec](docs/spec/copy-mode.md)
- **Store recovery** — rebuild local cache files from canonical manifests: [`store rebuild-index`](docs/reference/store-commands.md#store-rebuild-index)

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

- [Installation](docs/guide/installation.md) — installation methods, source builds, and Windows setup
- [Workflows](docs/guide/workflows.md) — start managing a local file and follow common recovery procedures

See [docs/index.md](docs/index.md) for the full documentation set.

## License

MIT
