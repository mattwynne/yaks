// Git ref sync adapter - synchronizes yaks via git refs/notes/yaks

use crate::domain::ports::SyncPort;
use anyhow::{Context, Result};
use git2::{Oid, Repository};
use std::path::{Path, PathBuf};

pub struct GitRefSync {
    repo: Repository,
    yaks_path: PathBuf,
}

impl GitRefSync {
    pub fn new(repo_root: &Path, yaks_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_root).with_context(|| {
            format!(
                "Failed to open git repository at {}",
                repo_root.display()
            )
        })?;

        Ok(Self {
            repo,
            yaks_path: yaks_path.to_path_buf(),
        })
    }

    // Fetch refs/notes/yaks from origin into refs/remotes/origin/yaks
    fn fetch_remote(&self) -> Result<()> {
        // Try to fetch, but don't fail if remote doesn't exist or has no yaks ref yet
        let refspec = "refs/notes/yaks:refs/remotes/origin/yaks";

        if let Ok(mut remote) = self.repo.find_remote("origin") {
            let _ = remote.fetch(&[refspec], None, None);
        }

        Ok(())
    }

    // Get the OID of refs/remotes/origin/yaks if it exists
    fn get_remote_ref(&self) -> Result<Option<Oid>> {
        match self.repo.refname_to_id("refs/remotes/origin/yaks") {
            Ok(oid) => Ok(Some(oid)),
            Err(_) => Ok(None),
        }
    }

    // Get the OID of refs/notes/yaks if it exists
    fn get_local_ref(&self) -> Result<Option<Oid>> {
        match self.repo.refname_to_id("refs/notes/yaks") {
            Ok(oid) => Ok(Some(oid)),
            Err(_) => Ok(None),
        }
    }

    // Build a tree from .yaks directory
    fn build_tree_from_yaks(&self) -> Result<Oid> {
        let mut index = git2::Index::new()?;

        if self.yaks_path.exists() {
            for entry in walkdir::WalkDir::new(&self.yaks_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let relative = path.strip_prefix(&self.yaks_path)?;
                let contents = std::fs::read(path)?;

                // Create blob from file contents
                let oid = self.repo.blob(&contents)?;

                // Add to index
                let index_entry = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644, // regular file
                    uid: 0,
                    gid: 0,
                    file_size: contents.len() as u32,
                    id: oid,
                    flags: 0,
                    flags_extended: 0,
                    path: relative.to_str().unwrap().as_bytes().to_vec(),
                };
                index.add(&index_entry)?;
            }
        }

        let tree_oid = index.write_tree_to(&self.repo)?;
        Ok(tree_oid)
    }

    // Check if local .yaks has uncommitted changes compared to the ref
    fn has_uncommitted_changes(&self, local_ref: Option<Oid>) -> Result<bool> {
        // If no local ref exists, any content in .yaks is uncommitted
        let Some(local_oid) = local_ref else {
            return Ok(self.yaks_path.exists() && self.yaks_path.read_dir()?.next().is_some());
        };

        // Compare .yaks directory with the tree at local_ref
        let commit = self.repo.find_commit(local_oid)?;
        let ref_tree_oid = commit.tree_id();

        // Build tree from current .yaks directory
        let yaks_tree_oid = self.build_tree_from_yaks()?;

        Ok(yaks_tree_oid != ref_tree_oid)
    }

    // Commit current .yaks directory to refs/notes/yaks
    fn commit_local_changes(&self, message: &str) -> Result<Oid> {
        let tree_oid = self.build_tree_from_yaks()?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Get parent commit if refs/notes/yaks exists
        let parent = self
            .get_local_ref()?
            .and_then(|oid| self.repo.find_commit(oid).ok());

        let parents: Vec<_> = parent.iter().collect();

        // Create commit
        let sig = self.repo.signature()?;
        let oid = self.repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;

        Ok(oid)
    }

    // Commit current .yaks directory as a merge commit with two parents
    fn commit_merge(&self, message: &str, parent1: Oid, parent2: Oid) -> Result<Oid> {
        let tree_oid = self.build_tree_from_yaks()?;
        let tree = self.repo.find_tree(tree_oid)?;

        let commit1 = self.repo.find_commit(parent1)?;
        let commit2 = self.repo.find_commit(parent2)?;
        let parents = vec![&commit1, &commit2];

        // Create merge commit
        let sig = self.repo.signature()?;
        let oid = self.repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;

        Ok(oid)
    }

    // Extract .yaks directory from refs/notes/yaks
    fn extract_to_working_dir(&self) -> Result<()> {
        // Remove existing .yaks
        if self.yaks_path.exists() {
            std::fs::remove_dir_all(&self.yaks_path)?;
        }
        std::fs::create_dir_all(&self.yaks_path)?;

        // Extract from refs/notes/yaks if it exists
        if let Some(oid) = self.get_local_ref()? {
            let commit = self.repo.find_commit(oid)?;
            let tree = commit.tree()?;

            // Walk tree and extract files
            tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
                if entry.kind() == Some(git2::ObjectType::Blob) {
                    let full_path = if dir.is_empty() {
                        entry.name().unwrap_or("").to_string()
                    } else {
                        format!("{}/{}", dir, entry.name().unwrap_or(""))
                    };

                    if let Ok(blob) = entry
                        .to_object(&self.repo)
                        .and_then(|obj| obj.peel_to_blob())
                    {
                        let file_path = self.yaks_path.join(&full_path);
                        if let Some(parent) = file_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&file_path, blob.content());
                    }
                }
                git2::TreeWalkResult::Ok
            })?;
        }

        Ok(())
    }

    // Merge remote ref into local ref at git level
    fn merge_refs(&self, local_ref: Oid, remote_ref: Oid) -> Result<Oid> {
        // Check for fast-forward cases
        if self.repo.graph_descendant_of(local_ref, remote_ref)? {
            // Local is ahead, keep it
            return Ok(local_ref);
        }

        if self.repo.graph_descendant_of(remote_ref, local_ref)? {
            // Remote is ahead, fast-forward to it
            self.repo
                .reference("refs/notes/yaks", remote_ref, true, "sync: fast-forward")?;
            return Ok(remote_ref);
        }

        // Neither is ahead - do a merge
        let local_commit = self.repo.find_commit(local_ref)?;
        let remote_commit = self.repo.find_commit(remote_ref)?;

        // Try to find merge base - may not exist for unrelated histories
        let merge_base = self.repo.merge_base(local_ref, remote_ref).ok();

        let mut index = if let Some(merge_base_oid) = merge_base {
            // Normal 3-way merge with a common ancestor
            let merge_base_commit = self.repo.find_commit(merge_base_oid)?;
            self.repo.merge_trees(
                &merge_base_commit.tree()?,
                &local_commit.tree()?,
                &remote_commit.tree()?,
                None,
            )?
        } else {
            // Unrelated histories - merge without a base (allow unrelated histories)
            // Use an empty tree as the base
            let empty_tree = self.repo.treebuilder(None)?.write()?;
            let empty_tree_obj = self.repo.find_tree(empty_tree)?;
            self.repo.merge_trees(
                &empty_tree_obj,
                &local_commit.tree()?,
                &remote_commit.tree()?,
                None,
            )?
        };

        // Resolve conflicts automatically
        if index.has_conflicts() {
            // For yaks, we use a "keep both sides, prefer local for modifications" strategy
            // - If a yak is deleted on one side and modified on the other, keep the modified version
            // - If a yak is modified on both sides, prefer local (last-write-wins at file level within the yak)

            // Collect conflicts first to avoid borrowing issues
            let conflicts: Vec<_> = index.conflicts()?.flatten().collect();

            for conflict in conflicts {
                let path = if let Some(ref theirs) = conflict.their {
                    std::str::from_utf8(&theirs.path)?
                } else if let Some(ref ours) = conflict.our {
                    std::str::from_utf8(&ours.path)?
                } else if let Some(ref ancestor) = conflict.ancestor {
                    std::str::from_utf8(&ancestor.path)?
                } else {
                    continue;
                };

                // Determine which side to keep
                let (keep_oid, keep_mode) = match (&conflict.our, &conflict.their) {
                    (None, Some(theirs)) => {
                        // Deleted locally (ours), exists remotely (theirs) - keep remote
                        (Some(theirs.id), Some(theirs.mode))
                    }
                    (Some(ours), None) => {
                        // Exists locally (ours), deleted remotely (theirs) - keep local
                        (Some(ours.id), Some(ours.mode))
                    }
                    (Some(ours), Some(_theirs)) => {
                        // Modified on both sides - prefer local (last-write-wins)
                        (Some(ours.id), Some(ours.mode))
                    }
                    (None, None) => {
                        // Both deleted - remove it
                        (None, None)
                    }
                };

                // Remove the conflict
                index.remove_path(std::path::Path::new(path))?;

                // Add the resolved version
                if let (Some(oid), Some(mode)) = (keep_oid, keep_mode) {
                    let index_entry = git2::IndexEntry {
                        ctime: git2::IndexTime::new(0, 0),
                        mtime: git2::IndexTime::new(0, 0),
                        dev: 0,
                        ino: 0,
                        mode,
                        uid: 0,
                        gid: 0,
                        file_size: 0,
                        id: oid,
                        flags: 0,
                        flags_extended: 0,
                        path: path.as_bytes().to_vec(),
                    };
                    index.add(&index_entry)?;
                }
            }
        }

        let tree_oid = index.write_tree_to(&self.repo)?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Create merge commit
        let sig = self.repo.signature()?;
        let merge_oid = self.repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Merge yaks",
            &tree,
            &[&local_commit, &remote_commit],
        )?;

        Ok(merge_oid)
    }

    // Push refs/notes/yaks to origin
    fn push_to_remote(&self) -> Result<()> {
        if self.get_local_ref()?.is_none() {
            // Nothing to push
            return Ok(());
        }

        if let Ok(mut remote) = self.repo.find_remote("origin") {
            let refspec = "refs/notes/yaks:refs/notes/yaks";
            let _ = remote.push(&[refspec], None);
        }

        Ok(())
    }

    // Merge remote files into local .yaks directory (last-write-wins at yak level)
    fn merge_remote_into_local_yaks(&self, remote_ref: Oid) -> Result<()> {
        let temp_dir = tempfile::tempdir()?;

        // Extract remote to temp
        let commit = self.repo.find_commit(remote_ref)?;
        let tree = commit.tree()?;

        tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let full_path = if dir.is_empty() {
                    entry.name().unwrap_or("").to_string()
                } else {
                    format!("{}/{}", dir, entry.name().unwrap_or(""))
                };

                if let Ok(blob) = entry
                    .to_object(&self.repo)
                    .and_then(|obj| obj.peel_to_blob())
                {
                    let file_path = temp_dir.path().join(&full_path);
                    if let Some(parent) = file_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&file_path, blob.content());
                }
            }
            git2::TreeWalkResult::Ok
        })?;

        // Find all yak directories that exist locally
        let local_yaks: std::collections::HashSet<String> = if self.yaks_path.exists() {
            walkdir::WalkDir::new(&self.yaks_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
                .filter_map(|e| {
                    e.path()
                        .strip_prefix(&self.yaks_path)
                        .ok()
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                })
                .collect()
        } else {
            std::collections::HashSet::new()
        };

        // For each local yak, remove it from temp and copy the entire local version
        for yak_name in &local_yaks {
            let temp_yak_dir = temp_dir.path().join(yak_name);
            if temp_yak_dir.exists() {
                std::fs::remove_dir_all(&temp_yak_dir)?;
            }

            let local_yak_dir = self.yaks_path.join(yak_name);
            if local_yak_dir.exists() {
                let dest_dir = temp_dir.path().join(yak_name);
                std::fs::create_dir_all(&dest_dir)?;

                // Copy all files from local yak
                for entry in walkdir::WalkDir::new(&local_yak_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let path = entry.path();
                    let relative = path.strip_prefix(&local_yak_dir)?;
                    let dest = dest_dir.join(relative);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(path, dest)?;
                }
            }
        }

        // Replace .yaks with merged content
        if self.yaks_path.exists() {
            std::fs::remove_dir_all(&self.yaks_path)?;
        }
        std::fs::create_dir_all(&self.yaks_path)?;

        for entry in walkdir::WalkDir::new(temp_dir.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let relative = path.strip_prefix(temp_dir.path())?;
            let dest = self.yaks_path.join(relative);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(path, dest)?;
        }

        Ok(())
    }
}

impl SyncPort for GitRefSync {
    fn sync(&self) -> Result<()> {
        // Step 1: Fetch remote
        self.fetch_remote()?;

        let remote_ref = self.get_remote_ref()?;
        let local_ref = self.get_local_ref()?;

        // Step 2: Handle uncommitted local changes
        let has_local_changes = self.has_uncommitted_changes(local_ref)?;

        let local_ref = if has_local_changes && remote_ref.is_some() {
            // We have local changes AND a remote - do yak-level merge then create merge commit
            self.merge_remote_into_local_yaks(remote_ref.unwrap())?;
            let merged_oid = if let Some(local_oid) = local_ref {
                self.commit_merge("Merge yaks", local_oid, remote_ref.unwrap())?
            } else {
                // No local ref, just commit with remote as parent
                self.commit_local_changes("sync")?
            };
            Some(merged_oid)
        } else if has_local_changes {
            // Local changes but no remote - just commit
            Some(self.commit_local_changes("sync")?)
        } else {
            // No local changes
            local_ref
        };

        // Step 3: Merge at git ref level (only if we didn't already create a merge commit)
        let already_merged = has_local_changes && remote_ref.is_some();

        if !already_merged {
            if let (Some(local_oid), Some(remote_oid)) = (local_ref, remote_ref) {
                if local_oid != remote_oid {
                    self.merge_refs(local_oid, remote_oid)?;
                }
            } else if let Some(remote_oid) = remote_ref {
                // No local ref, just use remote
                self.repo
                    .reference("refs/notes/yaks", remote_oid, true, "sync: use remote")?;
            }
        }

        // Step 4: Push to remote
        self.push_to_remote()?;

        // Step 5: Extract final result to .yaks
        self.extract_to_working_dir()?;

        // Cleanup: remove refs/remotes/origin/yaks
        if let Ok(mut ref_) = self.repo.find_reference("refs/remotes/origin/yaks") {
            let _ = ref_.delete();
        }

        Ok(())
    }
}
