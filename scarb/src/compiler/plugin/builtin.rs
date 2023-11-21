use anyhow::Result;
use cairo_lang_defs::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use std::ffi::OsString;
use std::sync::Arc;
// use cairo_lang_test_plugin::test_plugin_suite;

use crate::compiler::plugin::{CairoPlugin, CairoPluginInstance};
use crate::core::{PackageId, PackageName, SourceId};
use crate::internal::to_version::ToVersion;

pub struct BuiltinStarkNetPlugin;
impl CairoPlugin for BuiltinStarkNetPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::STARKNET,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinStarkNetPluginInstance))
    }
}

struct BuiltinStarkNetPluginInstance;
impl CairoPluginInstance for BuiltinStarkNetPluginInstance {
    fn plugin_suite(&self) -> PluginSuite {
        starknet_plugin_suite()
    }
}

pub struct BuiltinTestPlugin{
    lib: Arc<libloading::Library>,
}

impl BuiltinTestPlugin {
    pub fn new() -> Self {
        let library_path: OsString = std::env::var_os("SCARB_DYLIB").unwrap_or_else(|| {
            OsString::from(
                "/home/maciektr/Projects/scarb-plugins/dylib/target/debug/libplugin.so",
            )
        });
        println!("loading: {}", library_path.clone().to_string_lossy().to_string());
        let lib = unsafe { libloading::Library::new(library_path).unwrap() };
        Self { lib: Arc::new(lib) }
    }
}

impl CairoPlugin for BuiltinTestPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::TEST_PLUGIN,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinTestPluginInstance {lib: self.lib.clone()}))
    }
}

struct BuiltinTestPluginInstance {
        lib: Arc<libloading::Library>,
}

impl CairoPluginInstance for BuiltinTestPluginInstance {
    fn plugin_suite(&self) -> PluginSuite {
        let f = unsafe {
            self.lib
                .get::<fn() -> PluginSuite>(b"test_plugin_suite\0")
                .unwrap()
        };
        let p = f();
        dbg!(&p);
        p
    }
}
