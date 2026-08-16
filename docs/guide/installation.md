# Installation / Uninstallation

Install `shelfbox` using Homebrew, a pre-built binary, or a source build.

## Linux and macOS

### Homebrew

```sh
brew install masa-kjm/tap/shelfbox
```

### Pre-built Binary - Unix

```sh
curl -fsSL https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.sh | sh
```

The Unix installer uses `~/.local/bin` by default.

#### Advanced Installer Options

To install a specific version or use a custom directory:

```sh
# version format: vMAJOR.MINOR.PATCH
curl -fsSL https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.sh | VERSION=<version> sh
curl -fsSL https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.sh | INSTALL_DIR=/usr/local/bin sh
```

Linux installs use the musl binary by default for wider compatibility. To use the GNU libc binary instead:

```sh
curl -fsSL https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.sh | LINUX_LIBC=gnu sh
```

## Windows

> [!IMPORTANT]
> Before first use, Windows requires [additional setup](#required-setup).

### Pre-built Binary - Windows

```powershell
irm https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.ps1 | iex
```

The PowerShell installer uses `$Env:LOCALAPPDATA\Programs\shelfbox\bin` by default.

#### Advanced Installer Options

To install a specific version or use a custom directory, set `VERSION` or `INSTALL_DIR` before running the Windows installer:

```powershell
# version format: vMAJOR.MINOR.PATCH
$Env:VERSION = "<version>"
$Env:INSTALL_DIR = "$Env:USERPROFILE\bin"
irm https://raw.githubusercontent.com/masa-kjm/shelfbox/main/scripts/install.ps1 | iex
```

### Required Setup

Windows does not provide the parent-directory durability capability required by the default strict mutation policy. Before the first shelf mutation, opt in to the reduced-guarantee policy:

```powershell
shelfbox config set mutation_durability best-effort
```

> [!NOTE]
> The default materialization strategy uses symlinks, which require Developer Mode or an elevated shell on Windows. When symlink creation is unavailable, configure Copy mode before adding an item:
>
> ```powershell
> shelfbox config set materialization copy
> ```
>
> Copy mode selects a regular-file materialization; it does not change the durability policy. `best-effort` continues only when directory durability is unavailable and does not guarantee complete recovery after power loss or forced termination. See the [Copy mode specification](../spec/copy-mode.md) for its sync and recovery behavior.

## Build from Source

Building from source requires Git and Rust 1.75+.

Clone the repository, then install the CLI from its workspace:

```sh
git clone https://github.com/masa-kjm/shelfbox.git
cd shelfbox
cargo install --path crates/shelfbox
```

## Uninstallation

Before removing the executable or any data, record the active store and configuration locations:

```sh
shelfbox config get store
shelfbox config path
```

> [!IMPORTANT]
> The store contains the canonical content of managed items. Restore every item you want to keep with `shelfbox item restore <PATH>`, or back up the store, before deleting it. Removing a store without restoring its items permanently removes their canonical content and leaves their repository materializations unusable.

Remove the executable with the tool that installed it.
For Homebrew, run:

```sh
brew uninstall masa-kjm/tap/shelfbox
```

For a pre-built binary, remove the executable from the installation directory.  
The default locations are `~/.local/bin/shelfbox` on Linux and macOS, and `$Env:LOCALAPPDATA\Programs\shelfbox\bin\shelfbox.exe` on Windows. Use the custom `INSTALL_DIR` instead when you installed to a different directory.

After restoring or backing up the necessary items, you may remove the exact store directory reported by `shelfbox config get store`. You may also remove the configuration file reported by `shelfbox config path` if you no longer need your local shelfbox preferences. These data removals are optional and are independent of uninstalling the executable.

## Next Step

Follow [Start Managing a Local File](workflows.md#start-managing-a-local-file) to create your first managed item.
