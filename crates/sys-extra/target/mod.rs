use snafu::prelude::*;
use std::{env, env::VarError, ops::Deref};

type Result<T> = std::result::Result<T, Error>;

/// 目标三元组，默认 x86_64_unknown_linux_gnu
/// https://doc.rust-lang.org/rustc/platform-support.html
#[derive(Debug, Eq, PartialEq)]
pub struct TargetTriple {
    // 架构
    architecture: String,
    // 供应商
    vendor: String,
    // 操作系统
    system: String,
    // ABI
    environment: String,
}

impl Default for TargetTriple {
    fn default() -> Self {
        let target = target_triple::TARGET;
        Self::parse_from_str(target)
    }
}

impl TargetTriple {
    fn new(
        arch: impl AsRef<str>,
        ven: impl AsRef<str>,
        sys: impl AsRef<str>,
        env: impl AsRef<str>,
    ) -> Self {
        Self {
            architecture: arch.as_ref().to_owned(),
            vendor: ven.as_ref().to_owned(),
            system: sys.as_ref().to_owned(),
            environment: env.as_ref().to_owned(),
        }
    }

    fn parse_from_str(target: impl AsRef<str>) -> Self {
        let target = target.as_ref();
        let targets = target.split("-").collect::<Vec<_>>();
        let architecture = targets.first().map_or("", Deref::deref);
        let vendor = targets.get(1).map_or("", Deref::deref);
        let system = targets.get(2).map_or("", Deref::deref);
        let environment = targets.get(3).map_or("", Deref::deref);
        Self::new(architecture, vendor, system, environment)
    }

    /// 从环境变量中获取到目标三元组
    pub fn parse_from_env() -> Result<Self> {
        // target 样例 x86_64-unknown-linux-gnu
        let target = env::var("TARGET").context(NoEnvVarSnafu)?;
        let target = Self::parse_from_str(target);
        Ok(target)
    }

    /// 通过供应商判断，编译目标是否来自苹果的设备
    #[inline]
    pub fn is_apple(&self) -> bool {
        self.vendor == "apple"
    }

    /// 通过供应商和操作系统判断，编译目标是否是 Apple MacOS 系统
    #[inline]
    pub fn is_apple_darwin(&self) -> bool {
        self.is_apple() && self.system == "darwin"
    }

    /// 通过操作系统判断，编译目标是否是 Android 系统
    #[inline]
    pub fn is_android(&self) -> bool {
        self.system.contains("android")
    }

    /// 通过操作系统和架构判断，编译目标是否 aarch64 平台上的 Android 系统
    #[inline]
    pub fn is_aarch64_android(&self) -> bool {
        self.is_android() && self.architecture == "aarch64"
    }

    /// 通过操作系统和架构判断，编译目标是否 armv7 平台上的 Android 系统
    #[inline]
    pub fn is_armv7_android(&self) -> bool {
        self.is_android() && self.architecture == "armv7"
    }

    /// 通过操作系统和架构判断，编译目标是否 x86_64 平台上的 Android 系统
    #[inline]
    pub fn is_x86_64_android(&self) -> bool {
        self.is_android() && self.architecture == "x86_64"
    }

    /// 通过操作系统和架构判断，编译目标是否 i686 平台上的 Android 系统
    #[inline]
    pub fn is_i686_android(&self) -> bool {
        self.is_android() && self.architecture == "i686"
    }

    /// 通过操作系统判断，编译目标是否是 Linux 系统
    #[inline]
    pub fn is_linux(&self) -> bool {
        self.system == "linux"
    }

    /// 通过操作系统和架构判断，编译目标是否是 x86_64 平台的 Linux 系统
    #[inline]
    pub fn is_x86_64_linux(&self) -> bool {
        self.system == "linux" && self.architecture == "x86_64"
    }

    /// 通过操作系统和架构判断，编译目标是否是 aarch64 平台的 Linux 系统
    #[inline]
    pub fn is_aarch64_linux(&self) -> bool {
        self.system == "linux" && self.architecture == "aarch64"
    }

    /// 通过操作系统判断，编译目标是否是 Windows 系统
    #[inline]
    pub fn is_windows(&self) -> bool {
        self.system == "windows"
    }

    /// 通过操作系统和架构判断，编译目标是否是 i686 平台的 Windows 系统
    #[inline]
    pub fn is_i686_windows(&self) -> bool {
        self.system == "windows" && self.architecture == "i686"
    }

    /// 通过操作系统和架构判断，编译目标是否是 x86_64 平台的 Windows 系统
    #[inline]
    pub fn is_x86_64_windows(&self) -> bool {
        self.system == "windows" && self.architecture == "x86_64"
    }

    /// 通过操作系统和架构判断，编译目标是否是 aarch64 平台的 Windows 系统
    #[inline]
    pub fn is_aarch64_windows(&self) -> bool {
        self.system == "windows" && self.architecture == "aarch64"
    }

    /// 通过操作系统和 api 判断，编译目标是否是 Windows 系统，并且采用 MSVC 工具链编译
    #[inline]
    pub fn is_windows_msvc(&self) -> bool {
        self.is_windows() && self.environment == "msvc"
    }

    /// 通过 api 判断，采用 gnu 工具链编译
    #[inline]
    pub fn is_gnu(&self) -> bool {
        self.environment == "gnu"
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to get env var target!"))]
    NoEnvVar { source: VarError },
}
