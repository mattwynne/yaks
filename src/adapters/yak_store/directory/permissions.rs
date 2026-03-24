use anyhow::Result;
use std::fs;
use std::path::Path;

/// Make a file writable (owner write permission)
pub(super) fn make_writable(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut perms = fs::metadata(path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perms.mode();
        perms.set_mode(mode | 0o200); // add owner write
    }
    #[cfg(not(unix))]
    {
        perms.set_readonly(false);
    }
    fs::set_permissions(path, perms)?;
    Ok(())
}

/// Make a file read-only (remove all write bits)
pub(super) fn make_readonly(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(path, perms)?;
    Ok(())
}

/// Make a directory and all its files read-only (recursively)
pub(super) fn make_dir_readonly(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            make_dir_readonly(&path)?;
        } else {
            make_readonly(&path)?;
        }
    }
    make_readonly(dir)?;
    Ok(())
}

/// Recursively make a directory and all contents writable
pub(super) fn make_dir_writable_recursive(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    make_writable(dir)?;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            make_dir_writable_recursive(&path)?;
        } else {
            make_writable(&path)?;
        }
    }
    Ok(())
}
