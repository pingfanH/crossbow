#![cfg(target_os = "macos")]

use creator_tools::*;

#[test]
fn test_gen_apple_app() {
    let tempdir = tempfile::tempdir().unwrap();
    let dir = tempdir.path();
    let name = gen_minimal_project(dir).unwrap();

    // Creates target dir
    let target_dir = dir.join("target");
    std::fs::create_dir(&target_dir).unwrap();
    // Generate app folder
    let gen_apple_app = GenAppleApp::new(target_dir, name, Default::default(), Default::default());
    let app_dir = gen_apple_app.run().unwrap();
    // Check app dir
    assert_eq!(true, app_dir.exists());
}
