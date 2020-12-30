use crate::error::*;
use crate::types::AndroidTarget;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct AndroidNdk {
    ndk_path: PathBuf,
}

impl AndroidNdk {
    pub fn from_env(sdk_path: Option<&Path>) -> Result<Self> {
        let ndk_path = {
            let ndk_path = std::env::var("ANDROID_NDK_ROOT")
                .ok()
                .or_else(|| std::env::var("ANDROID_NDK_PATH").ok())
                .or_else(|| std::env::var("ANDROID_NDK_HOME").ok())
                .or_else(|| std::env::var("NDK_HOME").ok());
            // Default ndk installation path
            if ndk_path.is_none()
                && sdk_path.is_some()
                && sdk_path.as_ref().unwrap().join("ndk-bundle").exists()
            {
                sdk_path.unwrap().join("ndk-bundle")
            } else {
                PathBuf::from(ndk_path.ok_or(AndroidError::AndroidNdkNotFound)?)
            }
        };
        Ok(Self { ndk_path })
    }

    pub fn ndk_path(&self) -> &Path {
        &self.ndk_path
    }

    pub fn toolchain_dir(&self) -> Result<PathBuf> {
        let host_os = std::env::var("HOST").ok();
        let host_contains = |s| host_os.as_ref().map(|h| h.contains(s)).unwrap_or(false);
        let arch = if host_contains("linux") {
            "linux"
        } else if host_contains("macos") {
            "darwin"
        } else if host_contains("windows") {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            return match host_os {
                Some(host_os) => Err(AndroidError::UnsupportedHost(host_os)),
                _ => Err(AndroidError::UnsupportedTarget),
            }?;
        };
        let mut toolchain_dir = self
            .ndk_path
            .join("toolchains")
            .join("llvm")
            .join("prebuilt")
            .join(format!("{}-x86_64", arch));
        if !toolchain_dir.exists() {
            toolchain_dir.set_file_name(arch);
        }
        if !toolchain_dir.exists() {
            return Err(Error::PathNotFound(toolchain_dir));
        }
        Ok(toolchain_dir)
    }

    pub fn clang(&self, target: AndroidTarget, platform: u32) -> Result<(PathBuf, PathBuf)> {
        #[cfg(target_os = "windows")]
        let ext = ".cmd";
        #[cfg(not(target_os = "windows"))]
        let ext = "";
        let bin_name = format!("{}{}-clang", target.ndk_llvm_triple(), platform);
        let bin_path = self.toolchain_dir()?.join("bin");
        let clang = bin_path.join(format!("{}{}", &bin_name, ext));
        if !clang.exists() {
            return Err(Error::PathNotFound(clang));
        }
        let clang_pp = bin_path.join(format!("{}++{}", &bin_name, ext));
        if !clang_pp.exists() {
            return Err(Error::PathNotFound(clang_pp));
        }
        Ok((clang, clang_pp))
    }

    pub fn toolchain_bin(&self, bin: &str, build_target: AndroidTarget) -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        let ext = ".exe";
        #[cfg(not(target_os = "windows"))]
        let ext = "";
        let bin = self.toolchain_dir()?.join("bin").join(format!(
            "{}-{}{}",
            build_target.ndk_triple(),
            bin,
            ext
        ));
        if !bin.exists() {
            return Err(Error::PathNotFound(bin));
        }
        Ok(bin)
    }

    pub fn readelf(&self, build_target: AndroidTarget) -> Result<Command> {
        let readelf_path = self.toolchain_bin("readelf", build_target)?;
        Ok(Command::new(readelf_path))
    }

    pub fn sysroot_lib_dir(&self, build_target: AndroidTarget) -> Result<PathBuf> {
        let sysroot_lib_dir = self
            .toolchain_dir()?
            .join("sysroot")
            .join("usr")
            .join("lib")
            .join(build_target.ndk_triple());
        if !sysroot_lib_dir.exists() {
            return Err(Error::PathNotFound(sysroot_lib_dir));
        }
        Ok(sysroot_lib_dir)
    }

    pub fn sysroot_platform_lib_dir(
        &self,
        build_target: AndroidTarget,
        min_sdk_version: u32,
    ) -> Result<PathBuf> {
        let sysroot_lib_dir = self.sysroot_lib_dir(build_target)?;

        // Look for a platform <= min_sdk_version
        let mut tmp_platform = min_sdk_version;
        while tmp_platform > 1 {
            let path = sysroot_lib_dir.join(tmp_platform.to_string());
            if path.exists() {
                return Ok(path);
            }
            tmp_platform += 1;
        }

        // Look for the minimum API level supported by the NDK
        let mut tmp_platform = min_sdk_version;
        while tmp_platform < 100 {
            let path = sysroot_lib_dir.join(tmp_platform.to_string());
            if path.exists() {
                return Ok(path);
            }
            tmp_platform += 1;
        }

        Err(AndroidError::PlatformNotFound(min_sdk_version).into())
    }
}
