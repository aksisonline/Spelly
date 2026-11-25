"""Run inside the build container on a COPY of the gpui-pre-windows crate.

GPUI compiles its HLSL shaders in a build script that is only compiled when the *build host*
is Windows, so cross-compiling from macOS/Linux would skip it and fail. This makes the build
script decide from the *target* instead, and skips the Windows-registry SDK lookup (we point
GPUI_FXC_PATH at fxc running under Wine). Never applied to the repo or to normal builds.
"""
import re, sys

path = sys.argv[1] + "/build.rs"
s = open(path).read()

# 1. main(): decide by target + profile, not by host cfg
s = re.sub(r"fn main\(\) \{.*?\n\}\n", '''fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("PROFILE").as_deref() == Ok("release")
    {
        compile_shaders();
    }
}
''', s, count=1, flags=re.S)

# 2. compile the shader module on every host
s = s.replace('#[cfg(all(target_os = "windows", not(debug_assertions)))]\nmod shader_compilation', 'mod shader_compilation')
s = s.replace('#[cfg(all(target_os = "windows", not(debug_assertions)))]\nuse shader_compilation::compile_shaders;', 'use shader_compilation::compile_shaders;')

# 3. the registry-based SDK lookup only exists on a Windows host
s = s.replace('    pub fn find_latest_windows_sdk_binary(', '    #[cfg(windows)]\n    pub fn find_latest_windows_sdk_binary(', 1)
s = s.replace('    /// You can set the `GPUI_FXC_PATH`', '''    #[cfg(not(windows))]
    pub fn find_latest_windows_sdk_binary(_binary: &str) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    /// You can set the `GPUI_FXC_PATH`''', 1)
open(path, "w").write(s)
print("patched", path)
