
    use super::*;
    use tempfile::TempDir;

    fn temp_pid_file() -> (TempDir, PidFile) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.pid");
        (dir, PidFile::new(path))
    }

    #[test]
    fn test_pid_file_new() {
        let pid = PidFile::new("/tmp/test.pid");
        assert_eq!(pid.path(), Path::new("/tmp/test.pid"));
        assert!(!pid.is_locked());
    }

    #[test]
    fn test_pid_file_not_exists() {
        let (_dir, pid) = temp_pid_file();
        assert!(!pid.exists());
        assert!(pid.read_pid().unwrap().is_none());
    }

    #[test]
    fn test_write_and_read_pid() {
        let (_dir, mut pid) = temp_pid_file();
        pid.write_pid_value(12345).unwrap();

        assert!(pid.exists());
        assert_eq!(pid.read_pid().unwrap(), Some(12345));
        assert!(pid.is_locked());
    }

    #[test]
    fn test_remove_pid_file() {
        let (_dir, mut pid) = temp_pid_file();
        pid.write_pid_value(12345).unwrap();
        assert!(pid.exists());

        pid.remove().unwrap();
        assert!(!pid.exists());
        assert!(!pid.is_locked());
    }

    #[test]
    fn test_remove_nonexistent() {
        let (_dir, mut pid) = temp_pid_file();
        // Should not error on removing non-existent file
        assert!(pid.remove().is_ok());
    }

    #[test]
    fn test_try_acquire_new() {
        let (_dir, mut pid) = temp_pid_file();
        assert!(pid.try_acquire().is_ok());
        assert!(pid.is_locked());
    }

    #[test]
    fn test_try_acquire_stale() {
        let (_dir, mut pid) = temp_pid_file();
        // Write a fake PID that's unlikely to be running
        pid.write_pid_value(999999).unwrap();
        pid.locked = false; // Simulate not owning the lock

        // A new PidFile should be able to acquire it (stale)
        let _pid2 = PidFile::new(pid.path());
        // This test assumes PID 999999 is not running
        // In real tests, we might need to use a mock
    }

    #[test]
    fn test_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("subdir").join("deep").join("test.pid");
        let mut pid = PidFile::new(path.clone());

        pid.write_pid_value(12345).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_is_process_running_current() {
        let current_pid = std::process::id();
        assert!(PidFile::is_process_running(current_pid));
    }

    #[test]
    fn test_force_remove() {
        let (_dir, mut pid) = temp_pid_file();
        pid.write_pid_value(12345).unwrap();

        let mut another = PidFile::new(pid.path());
        another.force_remove().unwrap();
        assert!(!pid.exists());
    }

    #[test]
    fn test_drop_removes_locked_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.pid");

        {
            let mut pid = PidFile::new(path.clone());
            pid.write_pid_value(12345).unwrap();
            assert!(path.exists());
        } // PidFile dropped here

        // File should be removed after drop
        assert!(!path.exists());
    }
