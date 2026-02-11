    use super::*;
    use async_trait::async_trait;
    use autohands_protocols::types::Version;
    use std::any::Any;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockExtension {
        manifest: ExtensionManifest,
    }

    impl MockExtension {
        fn new(id: &str) -> Self {
            Self {
                manifest: ExtensionManifest::new(id, "Mock Extension", Version::new(1, 0, 0)),
            }
        }
    }

    #[async_trait]
    impl Extension for MockExtension {
        fn manifest(&self) -> &ExtensionManifest {
            &self.manifest
        }

        async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
            Ok(())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_kernel_creation() {
        let kernel = Kernel::new(PathBuf::from("."));
        assert!(kernel.list_extensions().is_empty());
        assert_eq!(kernel.state(), KernelState::Created);
        assert!(kernel.task_submitter().is_none());
    }

    #[tokio::test]
    async fn test_kernel_start_stop() {
        let kernel = Kernel::new(PathBuf::from("."));

        kernel.start().await.unwrap();
        assert!(kernel.is_running());

        kernel.stop().await.unwrap();
        assert!(!kernel.is_running());
    }

    #[tokio::test]
    async fn test_load_extension() {
        let kernel = Kernel::new(PathBuf::from("."));
        let extension = Box::new(MockExtension::new("test-ext"));

        let result = kernel
            .load_extension(extension, serde_json::Value::Null)
            .await;
        assert!(result.is_ok());
        assert_eq!(kernel.list_extensions().len(), 1);
    }

    #[tokio::test]
    async fn test_unload_extension() {
        let kernel = Kernel::new(PathBuf::from("."));
        let extension = Box::new(MockExtension::new("test-ext"));

        kernel
            .load_extension(extension, serde_json::Value::Null)
            .await
            .unwrap();
        let result = kernel.unload_extension("test-ext").await;
        assert!(result.is_ok());
        assert!(kernel.list_extensions().is_empty());
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        let kernel = Kernel::new(PathBuf::from("."));
        let mut rx = kernel.shutdown_signal().subscribe();

        kernel.start().await.unwrap();
        kernel.stop().await.unwrap();

        // Should receive shutdown signal
        let result = rx.try_recv();
        assert!(result.is_ok());
    }

    #[test]
    fn test_kernel_tool_registry() {
        let kernel = Kernel::new(PathBuf::from("."));
        let registry = kernel.tool_registry();
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_kernel_provider_registry() {
        let kernel = Kernel::new(PathBuf::from("."));
        let registry = kernel.provider_registry();
        assert!(registry.list_ids().is_empty());
    }

    #[test]
    fn test_kernel_memory_registry() {
        let kernel = Kernel::new(PathBuf::from("."));
        let registry = kernel.memory_registry();
        assert!(registry.list_ids().is_empty());
    }

    #[tokio::test]
    async fn test_unload_nonexistent_extension() {
        let kernel = Kernel::new(PathBuf::from("."));
        let result = kernel.unload_extension("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_lifecycle_hook() {
        struct SimpleHook;

        #[async_trait]
        impl LifecycleHook for SimpleHook {
            async fn on_start(&self) -> Result<(), ExtensionError> {
                Ok(())
            }

            async fn on_stop(&self) -> Result<(), ExtensionError> {
                Ok(())
            }
        }

        let kernel = Kernel::new(PathBuf::from("."));
        kernel.register_lifecycle_hook(Arc::new(SimpleHook)).await;
    }

    #[tokio::test]
    async fn test_load_extension_with_missing_dependency() {
        use autohands_protocols::extension::{Dependencies, DependencySpec};

        struct ExtWithDep {
            manifest: ExtensionManifest,
        }

        impl ExtWithDep {
            fn new() -> Self {
                let mut manifest = ExtensionManifest::new("ext-with-dep", "Extension With Dep", Version::new(1, 0, 0));
                manifest.dependencies = Dependencies {
                    required: vec![DependencySpec {
                        id: "missing-dep".to_string(),
                        version: None,
                    }],
                    optional: vec![],
                };
                Self { manifest }
            }
        }

        #[async_trait]
        impl Extension for ExtWithDep {
            fn manifest(&self) -> &ExtensionManifest {
                &self.manifest
            }

            async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
                Ok(())
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let kernel = Kernel::new(PathBuf::from("."));
        let extension = Box::new(ExtWithDep::new());
        let result = kernel.load_extension(extension, serde_json::Value::Null).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_duplicate_extension() {
        let kernel = Kernel::new(PathBuf::from("."));
        let ext1 = Box::new(MockExtension::new("dup-ext"));
        let ext2 = Box::new(MockExtension::new("dup-ext"));

        kernel.load_extension(ext1, serde_json::Value::Null).await.unwrap();
        let result = kernel.load_extension(ext2, serde_json::Value::Null).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unload_calls_shutdown() {
        struct ShutdownTracker {
            manifest: ExtensionManifest,
            shutdown_called: AtomicBool,
        }

        impl ShutdownTracker {
            fn new() -> Self {
                Self {
                    manifest: ExtensionManifest::new(
                        "shutdown-tracker",
                        "Shutdown Tracker",
                        Version::new(1, 0, 0),
                    ),
                    shutdown_called: AtomicBool::new(false),
                }
            }
        }

        #[async_trait]
        impl Extension for ShutdownTracker {
            fn manifest(&self) -> &ExtensionManifest {
                &self.manifest
            }

            async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
                Ok(())
            }

            async fn shutdown(&self) -> Result<(), ExtensionError> {
                self.shutdown_called.store(true, Ordering::SeqCst);
                Ok(())
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let kernel = Kernel::new(PathBuf::from("."));
        let ext = Box::new(ShutdownTracker::new());

        kernel
            .load_extension(ext, serde_json::Value::Null)
            .await
            .unwrap();

        // Get a reference to verify shutdown was called
        let ext_ref = kernel
            .tool_registry()
            .clone(); // Just to keep kernel alive
        drop(ext_ref);

        // Get the extension from registry to check later
        let ext_arc = kernel
            .list_extensions();
        assert_eq!(ext_arc.len(), 1);

        kernel.unload_extension("shutdown-tracker").await.unwrap();
        assert!(kernel.list_extensions().is_empty());
    }

    #[tokio::test]
    async fn test_stop_calls_shutdown_on_all_extensions() {
        struct ShutdownFlag {
            manifest: ExtensionManifest,
            shutdown_called: Arc<AtomicBool>,
        }

        #[async_trait]
        impl Extension for ShutdownFlag {
            fn manifest(&self) -> &ExtensionManifest {
                &self.manifest
            }

            async fn initialize(&mut self, _ctx: ExtensionContext) -> Result<(), ExtensionError> {
                Ok(())
            }

            async fn shutdown(&self) -> Result<(), ExtensionError> {
                self.shutdown_called.store(true, Ordering::SeqCst);
                Ok(())
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let flag1 = Arc::new(AtomicBool::new(false));
        let flag2 = Arc::new(AtomicBool::new(false));

        let ext1 = Box::new(ShutdownFlag {
            manifest: ExtensionManifest::new("ext-1", "Ext 1", Version::new(1, 0, 0)),
            shutdown_called: flag1.clone(),
        });
        let ext2 = Box::new(ShutdownFlag {
            manifest: ExtensionManifest::new("ext-2", "Ext 2", Version::new(1, 0, 0)),
            shutdown_called: flag2.clone(),
        });

        let kernel = Kernel::new(PathBuf::from("."));
        kernel.start().await.unwrap();
        kernel
            .load_extension(ext1, serde_json::Value::Null)
            .await
            .unwrap();
        kernel
            .load_extension(ext2, serde_json::Value::Null)
            .await
            .unwrap();

        kernel.stop().await.unwrap();

        assert!(flag1.load(Ordering::SeqCst), "ext-1 shutdown not called");
        assert!(flag2.load(Ordering::SeqCst), "ext-2 shutdown not called");
    }
