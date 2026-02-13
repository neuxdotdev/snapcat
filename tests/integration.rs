use snapcat::{snapcat, SnapcatBuilder};
use std::fs;
use tempfile::tempdir;
#[test]
fn integration_full_flow() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn test() {}").unwrap();
    let options = SnapcatBuilder::new(dir.path())
        .include_file_size(true)
        .build();
    let result = snapcat(options).unwrap();
    assert!(result.tree.contains("main.rs"));
    assert_eq!(result.files.len(), 2);
    for file in result.files {
        assert!(file.size.is_some());
    }
}
