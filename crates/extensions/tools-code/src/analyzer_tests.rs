    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(&PathBuf::from("test.rs")), Some("rust".to_string()));
        assert_eq!(detect_language(&PathBuf::from("test.py")), Some("python".to_string()));
        assert_eq!(detect_language(&PathBuf::from("test.js")), Some("javascript".to_string()));
        assert_eq!(detect_language(&PathBuf::from("test.unknown")), None);
    }

    #[test]
    fn test_element_type_serialize() {
        let e = ElementType::Function;
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, "\"function\"");
    }

    #[test]
    fn test_analyze_rust_function() {
        let code = "pub fn my_function(x: i32) -> i32 { x }";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::Function);
    }

    #[test]
    fn test_analyze_rust_struct() {
        let code = "pub struct MyStruct {\n    field: i32,\n}";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::Struct);
        assert_eq!(elements[0].name, "MyStruct");
    }

    #[test]
    fn test_analyze_rust_enum() {
        let code = "pub enum MyEnum { A, B, C }";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::Enum);
    }

    #[test]
    fn test_analyze_rust_trait() {
        let code = "pub trait MyTrait { fn method(&self); }";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::Trait);
    }

    #[test]
    fn test_file_analysis_serialization() {
        let analysis = FileAnalysis {
            path: "test.rs".to_string(),
            language: "rust".to_string(),
            elements: vec![],
            imports: vec!["std::io".to_string()],
            line_count: 10,
        };
        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("test.rs"));
    }

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(detect_language(&PathBuf::from("app.ts")), Some("typescript".to_string()));
        assert_eq!(detect_language(&PathBuf::from("app.tsx")), Some("typescript-react".to_string()));
        assert_eq!(detect_language(&PathBuf::from("app.mts")), Some("typescript".to_string()));
    }

    #[test]
    fn test_detect_language_javascript() {
        assert_eq!(detect_language(&PathBuf::from("app.js")), Some("javascript".to_string()));
        assert_eq!(detect_language(&PathBuf::from("app.jsx")), Some("javascript-react".to_string()));
        assert_eq!(detect_language(&PathBuf::from("app.mjs")), Some("javascript".to_string()));
    }

    #[test]
    fn test_detect_language_go() {
        assert_eq!(detect_language(&PathBuf::from("main.go")), Some("go".to_string()));
    }

    #[test]
    fn test_detect_language_java_kotlin() {
        assert_eq!(detect_language(&PathBuf::from("Main.java")), Some("java".to_string()));
        assert_eq!(detect_language(&PathBuf::from("Main.kt")), Some("kotlin".to_string()));
        assert_eq!(detect_language(&PathBuf::from("build.kts")), Some("kotlin".to_string()));
    }

    #[test]
    fn test_detect_language_c_cpp() {
        assert_eq!(detect_language(&PathBuf::from("main.c")), Some("c".to_string()));
        assert_eq!(detect_language(&PathBuf::from("main.cpp")), Some("cpp".to_string()));
        assert_eq!(detect_language(&PathBuf::from("main.cc")), Some("cpp".to_string()));
        assert_eq!(detect_language(&PathBuf::from("main.cxx")), Some("cpp".to_string()));
        assert_eq!(detect_language(&PathBuf::from("header.h")), Some("c-header".to_string()));
        assert_eq!(detect_language(&PathBuf::from("header.hpp")), Some("c-header".to_string()));
    }

    #[test]
    fn test_detect_language_ruby_php() {
        assert_eq!(detect_language(&PathBuf::from("app.rb")), Some("ruby".to_string()));
        assert_eq!(detect_language(&PathBuf::from("index.php")), Some("php".to_string()));
    }

    #[test]
    fn test_detect_language_csharp_swift() {
        assert_eq!(detect_language(&PathBuf::from("Program.cs")), Some("csharp".to_string()));
        assert_eq!(detect_language(&PathBuf::from("App.swift")), Some("swift".to_string()));
    }

    #[test]
    fn test_detect_language_functional() {
        assert_eq!(detect_language(&PathBuf::from("main.scala")), Some("scala".to_string()));
        assert_eq!(detect_language(&PathBuf::from("core.clj")), Some("clojure".to_string()));
        assert_eq!(detect_language(&PathBuf::from("app.ex")), Some("elixir".to_string()));
        assert_eq!(detect_language(&PathBuf::from("app.exs")), Some("elixir".to_string()));
        assert_eq!(detect_language(&PathBuf::from("server.erl")), Some("erlang".to_string()));
        assert_eq!(detect_language(&PathBuf::from("Main.hs")), Some("haskell".to_string()));
        assert_eq!(detect_language(&PathBuf::from("main.ml")), Some("ocaml".to_string()));
        assert_eq!(detect_language(&PathBuf::from("main.mli")), Some("ocaml".to_string()));
    }

    #[test]
    fn test_detect_language_scripting() {
        assert_eq!(detect_language(&PathBuf::from("script.lua")), Some("lua".to_string()));
        assert_eq!(detect_language(&PathBuf::from("script.sh")), Some("shell".to_string()));
        assert_eq!(detect_language(&PathBuf::from("script.bash")), Some("shell".to_string()));
    }

    #[test]
    fn test_detect_language_data_formats() {
        assert_eq!(detect_language(&PathBuf::from("data.json")), Some("json".to_string()));
        assert_eq!(detect_language(&PathBuf::from("config.yaml")), Some("yaml".to_string()));
        assert_eq!(detect_language(&PathBuf::from("config.yml")), Some("yaml".to_string()));
        assert_eq!(detect_language(&PathBuf::from("Cargo.toml")), Some("toml".to_string()));
        assert_eq!(detect_language(&PathBuf::from("data.xml")), Some("xml".to_string()));
        assert_eq!(detect_language(&PathBuf::from("query.sql")), Some("sql".to_string()));
    }

    #[test]
    fn test_detect_language_web() {
        assert_eq!(detect_language(&PathBuf::from("index.html")), Some("html".to_string()));
        assert_eq!(detect_language(&PathBuf::from("index.htm")), Some("html".to_string()));
        assert_eq!(detect_language(&PathBuf::from("style.css")), Some("css".to_string()));
        assert_eq!(detect_language(&PathBuf::from("style.scss")), Some("scss".to_string()));
        assert_eq!(detect_language(&PathBuf::from("style.sass")), Some("scss".to_string()));
    }

    #[test]
    fn test_detect_language_markdown() {
        assert_eq!(detect_language(&PathBuf::from("README.md")), Some("markdown".to_string()));
        assert_eq!(detect_language(&PathBuf::from("doc.markdown")), Some("markdown".to_string()));
    }

    #[test]
    fn test_detect_language_no_extension() {
        assert_eq!(detect_language(&PathBuf::from("Makefile")), None);
        assert_eq!(detect_language(&PathBuf::from("")), None);
    }

    #[test]
    fn test_analyze_rust_module() {
        let code = "mod utils;\npub mod helpers;";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].element_type, ElementType::Module);
        assert_eq!(elements[0].name, "utils");
        assert_eq!(elements[1].name, "helpers");
    }

    #[test]
    fn test_analyze_rust_async_function() {
        // Current analyzer uses simple pattern matching, async fn not on same line as pub async fn
        let code = "async fn fetch_data() -> Result<()> { Ok(()) }";
        let elements = PatternAnalyzer::analyze_rust(code);
        // Analyzer does not yet detect async fn pattern (starts with "fn " or "pub fn ")
        // This test documents current behavior
        assert_eq!(elements.len(), 0);
    }

    #[test]
    fn test_analyze_rust_generic_struct() {
        let code = "pub struct Container<T> { item: T }";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::Struct);
        assert_eq!(elements[0].name, "Container");
    }

    #[test]
    fn test_analyze_rust_private_function() {
        let code = "fn private_helper() {}";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, ElementType::Function);
        assert_eq!(elements[0].name, "private_helper");
    }

    #[test]
    fn test_analyze_rust_private_struct() {
        let code = "struct PrivateStruct { field: i32 }";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].name, "PrivateStruct");
    }

    #[test]
    fn test_analyze_rust_multiple_elements() {
        let code = r#"
pub struct MyStruct {}
pub enum MyEnum { A, B }
pub trait MyTrait {}
pub fn my_function() {}
mod my_module;
"#;
        let elements = PatternAnalyzer::analyze_rust(code);
        assert_eq!(elements.len(), 5);
    }

    #[test]
    fn test_analyze_rust_empty_code() {
        let elements = PatternAnalyzer::analyze_rust("");
        assert!(elements.is_empty());
    }

    #[test]
    fn test_analyze_rust_comments_only() {
        let code = "// This is a comment\n/* Block comment */";
        let elements = PatternAnalyzer::analyze_rust(code);
        assert!(elements.is_empty());
    }

    #[test]
    fn test_code_element_fields() {
        let element = CodeElement {
            element_type: ElementType::Function,
            name: "test_fn".to_string(),
            start_line: 10,
            end_line: 20,
            signature: Some("fn test_fn() -> i32".to_string()),
            doc_comment: Some("/// Test function".to_string()),
        };
        assert_eq!(element.name, "test_fn");
        assert_eq!(element.start_line, 10);
        assert!(element.signature.is_some());
        assert!(element.doc_comment.is_some());
    }

    #[test]
    fn test_element_type_all_variants() {
        let types = vec![
            ElementType::Function,
            ElementType::Method,
            ElementType::Class,
            ElementType::Struct,
            ElementType::Enum,
            ElementType::Interface,
            ElementType::Trait,
            ElementType::Module,
            ElementType::Import,
            ElementType::Variable,
            ElementType::Constant,
            ElementType::Type,
            ElementType::Unknown,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_file_analysis_clone() {
        let analysis = FileAnalysis {
            path: "test.rs".to_string(),
            language: "rust".to_string(),
            elements: vec![],
            imports: vec![],
            line_count: 5,
        };
        let cloned = analysis.clone();
        assert_eq!(cloned.path, analysis.path);
        assert_eq!(cloned.line_count, analysis.line_count);
    }
