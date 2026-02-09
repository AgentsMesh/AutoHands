//! # AutoHands Macros
//!
//! Procedural macros for simplifying extension development.
//!
//! ## Available Macros
//!
//! - `#[extension]` - Define an extension
//! - `#[tool]` - Define a tool

use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct};

/// Extension attribute arguments.
#[derive(Debug, FromMeta)]
struct ExtensionArgs {
    id: String,
    name: String,
    #[darling(default)]
    version: Option<String>,
    #[darling(default)]
    description: Option<String>,
}

/// Tool attribute arguments.
#[derive(Debug, FromMeta)]
struct ToolArgs {
    id: String,
    name: String,
    description: String,
    #[darling(default)]
    risk_level: Option<String>,
}

/// Define an extension.
///
/// This macro generates the Extension trait implementation for a struct.
///
/// # Example
///
/// ```ignore
/// use autohands_macros::extension;
///
/// #[extension(
///     id = "my-extension",
///     name = "My Extension",
///     version = "0.1.0",
///     description = "A sample extension"
/// )]
/// struct MyExtension {
///     // extension fields
/// }
///
/// impl MyExtension {
///     pub fn new() -> Self {
///         Self {}
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn extension(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let args = match ExtensionArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    let id = &args.id;
    let name = &args.name;
    let version = args.version.unwrap_or_else(|| "0.1.0".to_string());
    let description = args.description.unwrap_or_default();

    // Parse version
    let version_parts: Vec<&str> = version.split('.').collect();
    let major: u32 = version_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor: u32 = version_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let patch: u32 = version_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let expanded = quote! {
        #input

        impl #struct_name {
            /// Get the extension manifest.
            pub fn manifest(&self) -> autohands_protocols::extension::ExtensionManifest {
                let mut manifest = autohands_protocols::extension::ExtensionManifest::new(
                    #id,
                    #name,
                    autohands_protocols::types::Version::new(#major, #minor, #patch),
                );
                manifest.description = #description.to_string();
                manifest
            }
        }
    };

    TokenStream::from(expanded)
}

/// Define a tool.
///
/// This macro generates a Tool struct from an async function.
///
/// # Example
///
/// ```ignore
/// use autohands_macros::tool;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct ReadFileParams {
///     path: String,
/// }
///
/// #[tool(
///     id = "read_file",
///     name = "Read File",
///     description = "Read contents of a file"
/// )]
/// async fn read_file(params: ReadFileParams) -> Result<String, String> {
///     std::fs::read_to_string(&params.path)
///         .map_err(|e| e.to_string())
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let args = match ToolArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;
    let fn_body = &input.block;

    // Generate struct name from function name (snake_case to PascalCase)
    let struct_name_str = fn_name
        .to_string()
        .split('_')
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            }
        })
        .collect::<String>()
        + "Tool";
    let struct_name = syn::Ident::new(&struct_name_str, fn_name.span());

    let id = &args.id;
    let name = &args.name;
    let description = &args.description;
    let risk_level = args.risk_level.unwrap_or_else(|| "Low".to_string());

    let risk_level_ident = match risk_level.to_lowercase().as_str() {
        "high" => quote! { autohands_protocols::types::RiskLevel::High },
        "medium" => quote! { autohands_protocols::types::RiskLevel::Medium },
        _ => quote! { autohands_protocols::types::RiskLevel::Low },
    };

    let expanded = quote! {
        /// Auto-generated tool struct for #fn_name.
        pub struct #struct_name {
            definition: autohands_protocols::tool::ToolDefinition,
        }

        impl #struct_name {
            /// Create a new instance of the tool.
            pub fn new() -> Self {
                let definition = autohands_protocols::tool::ToolDefinition::new(
                    #id,
                    #name,
                    #description,
                )
                .with_risk_level(#risk_level_ident);

                Self { definition }
            }
        }

        impl Default for #struct_name {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait::async_trait]
        impl autohands_protocols::tool::Tool for #struct_name {
            fn definition(&self) -> &autohands_protocols::tool::ToolDefinition {
                &self.definition
            }

            async fn execute(
                &self,
                params: serde_json::Value,
                ctx: autohands_protocols::tool::ToolContext,
            ) -> Result<autohands_protocols::tool::ToolResult, autohands_protocols::error::ToolError> {
                // Call the original function
                async fn inner(
                    params: serde_json::Value,
                    _ctx: autohands_protocols::tool::ToolContext,
                ) -> Result<autohands_protocols::tool::ToolResult, autohands_protocols::error::ToolError> {
                    let result: Result<String, String> = #fn_body;
                    match result {
                        Ok(content) => Ok(autohands_protocols::tool::ToolResult::success(content)),
                        Err(e) => Err(autohands_protocols::error::ToolError::ExecutionFailed(e)),
                    }
                }
                inner(params, ctx).await
            }
        }
    };

    TokenStream::from(expanded)
}
