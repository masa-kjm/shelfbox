use std::{
    io::ErrorKind,
    path::{Component, Path},
};

use crate::{
    error::{AppError, Result},
    failpoint::{self, Failpoint},
    fs::{platform, secure_transfer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParentDirMode {
    Default,
    Private,
}

pub(crate) fn ensure_parent_dir(path: &Path, mode: ParentDirMode) -> Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    ensure_dir(parent, mode)
}

/// Creates missing destination parents below `root` one component at a time.
///
/// Unlike [`ensure_parent_dir`], this never recurses through an existing
/// symlink, junction, reparse point, or other non-directory entry. Repair
/// calls this after planning has established that the destination is below its
/// repository root. Concurrent replacement of path components is outside the
/// local-repository threat model; each observed component is nevertheless
/// re-inspected after a creation attempt.
pub(crate) fn ensure_parent_dir_in_trusted_root(root: &Path, destination: &Path) -> Result<()> {
    let root_entry = platform::inspect_no_follow(root)?;
    if root_entry.kind != platform::EntryKind::Directory {
        return Err(AppError::UnsafeFilesystemEntry {
            path: root.to_path_buf(),
            reason: "trusted root is not a directory",
        });
    }

    let relative = secure_transfer::relative_path_in_trusted_root(root, destination)?;
    let parents = relative.parent().unwrap_or_else(|| Path::new(""));
    let mut current = root.to_path_buf();
    let mut created_parent = false;

    for component in parents.components() {
        let Component::Normal(name) = component else {
            return Err(AppError::UnsafeFilesystemEntry {
                path: destination.to_path_buf(),
                reason: "path contains a non-normal component",
            });
        };
        current.push(name);

        match platform::inspect_no_follow(&current) {
            Ok(entry) if entry.kind == platform::EntryKind::Directory => continue,
            Ok(_) => {
                return Err(AppError::UnsafeFilesystemEntry {
                    path: current,
                    reason: "intermediate component is not a directory",
                });
            }
            Err(AppError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => {
                if create_directory_component(&current)? {
                    created_parent = true;
                }
            }
            Err(error) => return Err(error),
        }

        let entry = platform::inspect_no_follow(&current)?;
        if entry.kind != platform::EntryKind::Directory {
            return Err(AppError::UnsafeFilesystemEntry {
                path: current,
                reason: "intermediate component is not a directory",
            });
        }
    }

    if created_parent {
        failpoint::after(Failpoint::ParentDirectoriesCreated)?;
    }
    Ok(())
}

fn create_directory_component(path: &Path) -> Result<bool> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(AppError::io(path, error)),
    }
}

pub(crate) fn ensure_dir(path: &Path, mode: ParentDirMode) -> Result<()> {
    match mode {
        ParentDirMode::Default => {
            std::fs::create_dir_all(path).map_err(|e| AppError::io(path, e))?;
        }
        ParentDirMode::Private => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(path)
                    .map_err(|e| AppError::io(path, e))?;
            }
            #[cfg(not(unix))]
            {
                std::fs::create_dir_all(path).map_err(|e| AppError::io(path, e))?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_parent_dir_creation_creates_missing_parent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/config.toml");

        ensure_parent_dir(&path, ParentDirMode::Default).unwrap();

        assert!(dir.path().join("nested").is_dir());
    }

    #[test]
    fn trusted_parent_dir_creation_creates_missing_parents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/deep/config.toml");

        ensure_parent_dir_in_trusted_root(dir.path(), &path).unwrap();

        assert!(dir.path().join("nested/deep").is_dir());
    }

    #[test]
    #[cfg(unix)]
    fn private_parent_dir_creation_uses_private_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("private/index.json");

        ensure_parent_dir(&path, ParentDirMode::Private).unwrap();

        let mode = std::fs::metadata(dir.path().join("private"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn trusted_parent_dir_creation_rejects_existing_symlink() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let nested = root.path().join("nested");
        symlink(outside.path(), &nested).unwrap();
        let destination = nested.join("deep/secret.txt");

        let error = ensure_parent_dir_in_trusted_root(root.path(), &destination).unwrap_err();

        assert!(matches!(error, AppError::UnsafeFilesystemEntry { .. }));
        assert!(!outside.path().join("deep").exists());
    }

    #[cfg(windows)]
    #[test]
    fn trusted_parent_dir_creation_rejects_existing_directory_symlink() {
        use std::os::windows::fs::symlink_dir;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let nested = root.path().join("nested");
        if symlink_dir(outside.path(), &nested).is_err() {
            return;
        }
        let destination = nested.join("deep/secret.txt");

        let error = ensure_parent_dir_in_trusted_root(root.path(), &destination).unwrap_err();

        assert!(matches!(error, AppError::UnsafeFilesystemEntry { .. }));
        assert!(!outside.path().join("deep").exists());
    }
}
