use crate::core::{Config, Package};
use crate::flock::Filesystem;
use anyhow::Result;
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};
use camino::Utf8PathBuf;
use libloading::{library_filename, Library, Symbol};
use scarb_macro_interface::stable_abi::{StableProcMacroResult, StableTokenStream};
use scarb_macro_interface::{ProcMacroResult, TokenStream};
use std::fmt::Debug;

#[cfg(not(windows))]
use libloading::os::unix::Symbol as RawSymbol;
#[cfg(windows)]
use libloading::os::windows::Symbol as RawSymbol;

pub const PROC_MACRO_BUILD_PROFILE: &str = "release";

pub trait FromItemAst {
    fn from_item_ast(db: &dyn SyntaxGroup, item_ast: ast::ModuleItem) -> Self;
}

impl FromItemAst for TokenStream {
    fn from_item_ast(db: &dyn SyntaxGroup, item_ast: ast::ModuleItem) -> Self {
        let mut builder = PatchBuilder::new(db);
        builder.add_node(item_ast.as_syntax_node());
        Self::new(builder.code)
    }
}

trait SharedLibraryProvider {
    fn shared_lib_path(&self, config: &Config) -> Filesystem;
}

impl SharedLibraryProvider for Package {
    fn shared_lib_path(&self, config: &Config) -> Filesystem {
        let lib_name = library_filename(self.id.name.to_string());
        let lib_name = lib_name.to_string_lossy().to_string();
        let ident = format!("{}-{}", self.id.name, self.id.source_id.ident());
        config
            .dirs()
            .procedural_macros_dir()
            .child(ident)
            .child("target")
            .child(PROC_MACRO_BUILD_PROFILE)
            .child(lib_name)
    }
}

/// Representation of a single procedural macro.
///
/// This struct is a wrapper around a shared library containing the procedural macro implementation.
/// It is responsible for loading the shared library and providing a safe interface for code expansion.
#[non_exhaustive]
pub struct ProcMacroInstance {
    plugin: Plugin,
}

impl Debug for ProcMacroInstance {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl ProcMacroInstance {
    /// Load shared library
    pub fn try_new(package: Package, config: &Config) -> Result<Self> {
        let lib_path = package.shared_lib_path(config);
        let plugin = unsafe { Plugin::try_new(lib_path.path_existent()?.to_path_buf())? };
        Ok(Self { plugin })
    }

    /// Apply expansion to token stream.
    pub(crate) fn generate_code(&self, token_stream: TokenStream) -> ProcMacroResult {
        let ffi_token_stream = unsafe { StableTokenStream::from_token_stream(token_stream) };
        let result = (self.plugin.vtable.expand)(ffi_token_stream);
        unsafe { result.into_proc_macro_result() }
    }
}

type ExpandCode = extern "C" fn(StableTokenStream) -> StableProcMacroResult;

struct VTableV0 {
    expand: RawSymbol<ExpandCode>,
}

impl VTableV0 {
    unsafe fn try_new(library: &Library) -> Result<VTableV0> {
        println!("Loading plugin API version 0...");

        let expand: Symbol<'_, ExpandCode> = library.get(b"expand\0")?;
        let expand = expand.into_raw();

        Ok(VTableV0 { expand })
    }
}

struct Plugin {
    #[allow(dead_code)]
    library: Library,
    vtable: VTableV0,
}

impl Plugin {
    unsafe fn try_new(library_name: Utf8PathBuf) -> Result<Plugin> {
        let library = Library::new(library_name)?;
        let vtable = VTableV0::try_new(&library)?;

        Ok(Plugin { library, vtable })
    }
}
