use snapcat::{
    snapcat,
    SnapcatBuilder,
    BinaryDetection,
};
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
#[test]
fn test_basic_scan() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("hello.txt");
    fs::write(&file_path, "hello world").unwrap();
    let options = SnapcatBuilder::new(dir.path())
        .binary_detection(BinaryDetection::None)
        .build();
    let result = snapcat(options).unwrap();
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].content, "hello world");
}
#[test]
fn test_ignore_patterns() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("b.log"), "b").unwrap();
    let options = SnapcatBuilder::new(dir.path())
        .ignore_patterns(vec!["*.log".into()])
        .build();
    let result = snapcat(options).unwrap();
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].path.ends_with("a.txt"));
}
#[test]
fn test_file_size_limit() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("big.txt");
    let mut f = File::create(&file_path).unwrap();
    write!(f, "{}", "A".repeat(5000)).unwrap();
    let options = SnapcatBuilder::new(dir.path())
        .file_size_limit(Some(100))
        .build();
    let result = snapcat(options).unwrap();
    assert!(result.files[0]
        .content
        .contains("File too large"));
}
#[test]
fn test_binary_detection_simple() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("bin.dat");
    fs::write(&file_path, vec![0,1,2,3]).unwrap();
    let options = SnapcatBuilder::new(dir.path())
        .binary_detection(BinaryDetection::Simple)
        .build();
    let result = snapcat(options).unwrap();
    assert!(result.files[0].is_binary);
}
