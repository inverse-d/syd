pub fn pull_from_remote(repo_path: &Path) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    
    // Fetch from remote
    let mut remote = repo.find_remote("origin")?;
    let mut fetch_options = FetchOptions::new();
    remote.fetch(&["main"], Some(&mut fetch_options), None)?;

    // Get remote main branch
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    // Perform merge
    let mut merge_options = MergeOptions::new();
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.0.is_fast_forward() {
        // Fast-forward merge
        let refname = "refs/heads/main";
        let mut reference = repo.find_reference(refname)?;
        reference.set_target(fetch_commit.id(), "Fast-forward")?;
        repo.set_head(refname)?;
        repo.checkout_head(Some(CheckoutBuilder::new().force()))?;
    }

    Ok(())
} 