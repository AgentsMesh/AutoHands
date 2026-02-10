    use super::*;
    use tempfile::TempDir;

    fn create_test_skill_dir(dir: &Path) {
        fs::create_dir_all(dir).unwrap();

        let skill_content = r#"---
id: test-package
name: Test Package Skill
version: 1.2.3
description: A skill for testing packaging
---

# Test Package Skill

This is a test skill for packaging.
"#;

        fs::write(dir.join("SKILL.markdown"), skill_content).unwrap();

        // Add some additional files
        fs::write(dir.join("README.md"), "# Test Skill\n\nReadme content.").unwrap();
    }

    #[test]
    fn test_package_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("test-skill");
        create_test_skill_dir(&skill_dir);

        // Pack
        let package_path = SkillPackager::pack(&skill_dir, temp_dir.path()).unwrap();
        assert!(package_path.exists());
        assert!(package_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".skill"));

        // Read package
        let package = SkillPackage::from_file(&package_path).unwrap();
        assert_eq!(package.version, VERSION);
        assert!(package.signature.is_none());
        assert!(!package.archive.is_empty());

        // Extract
        let extract_dir = temp_dir.path().join("extracted");
        let extracted = package.extract(&extract_dir).unwrap();

        // Verify extraction
        let extracted_skill = extracted.join("SKILL.markdown");
        assert!(extracted_skill.exists());
    }

    #[test]
    fn test_package_name_format() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("my-skill");
        create_test_skill_dir(&skill_dir);

        let package_path = SkillPackager::pack(&skill_dir, temp_dir.path()).unwrap();

        let file_name = package_path.file_name().unwrap().to_str().unwrap();
        assert_eq!(file_name, "test-package-1.2.3.skill");
    }

    #[test]
    fn test_install_package() {
        let temp_dir = TempDir::new().unwrap();
        let skill_dir = temp_dir.path().join("original");
        create_test_skill_dir(&skill_dir);

        // Pack
        let package_path = SkillPackager::pack(&skill_dir, temp_dir.path()).unwrap();

        // Install to new location
        let install_dir = temp_dir.path().join("installed");
        let installed = SkillPackager::install(&package_path, &install_dir).unwrap();

        // Verify
        let skill_file = installed.join("SKILL.markdown");
        assert!(skill_file.exists());
    }

    #[test]
    fn test_invalid_package_magic() {
        let temp_dir = TempDir::new().unwrap();
        let fake_package = temp_dir.path().join("fake.skill");
        fs::write(&fake_package, b"NOT A SKILL PACKAGE").unwrap();

        let result = SkillPackage::from_file(&fake_package);
        assert!(result.is_err());
    }

    #[test]
    fn test_package_with_signature() {
        let archive_data = vec![1, 2, 3, 4];
        let signature = [42u8; 64];

        let package = SkillPackage::new(archive_data).with_signature(signature);

        assert!(package.signature.is_some());
        assert_eq!(package.signature.unwrap(), signature);
    }

    #[test]
    fn test_pack_missing_skill_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_dir = temp_dir.path().join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        let result = SkillPackager::pack(&empty_dir, temp_dir.path());
        assert!(result.is_err());
    }
