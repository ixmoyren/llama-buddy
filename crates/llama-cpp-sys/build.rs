use anyhow::{Context, anyhow, bail};
use bindgen::{RustEdition, RustTarget};
use cmake::Config;
use fs::{copy, remove_file};
use glob::glob;
use std::{
    cmp::PartialEq,
    env, fs,
    fs::rename,
    path::{Path, PathBuf},
    process::Command,
};

/// llama.cpp 构建类型，默认是 Release
#[derive(Debug, Default, Eq, PartialEq)]
enum CMakeBuildType {
    Debug,
    #[default]
    Release,
    MinSizeRel,
    RelWithDebInfo,
}

impl CMakeBuildType {
    fn as_str(&self) -> &str {
        use CMakeBuildType::*;
        match self {
            Debug => "Debug",
            Release => "Release",
            MinSizeRel => "MinSizeRel",
            RelWithDebInfo => "RelWithDebInfo",
        }
    }
}

impl From<String> for CMakeBuildType {
    fn from(value: String) -> Self {
        let value = value.as_str();
        value.into()
    }
}

impl From<&str> for CMakeBuildType {
    fn from(value: &str) -> Self {
        use CMakeBuildType::*;
        match value {
            "Debug" => Debug,
            "Release" => Release,
            "MinSizeRel" => MinSizeRel,
            "RelWithDebInfo" => RelWithDebInfo,
            _ => panic!("This build type value is not supported!"),
        }
    }
}

/// 目标三元组，默认 x86_64_unknown_linux_gnu
/// https://doc.rust-lang.org/rustc/platform-support.html
#[derive(Debug, Eq, PartialEq)]
struct TargetTriple {
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
        Self::new("x86_64", "unknown", "linux", "gnu")
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

    /// 从环境变量中获取到目标三元组
    fn parse_from_env() -> anyhow::Result<Self> {
        // target 样例 x86_64-unknown-linux-gnu
        let target = env::var("TARGET").context("Failed to obtain env var target!")?;
        let targets = target.split("-").collect::<Vec<_>>();
        let architecture = targets.first().map_or("", |s| *s);
        let vendor = targets.get(1).map_or("", |s| *s);
        let system = targets.get(2).map_or("", |s| *s);
        let environment = targets.get(3).map_or("", |s| *s);
        Ok(Self::new(architecture, vendor, system, environment))
    }

    /// 通过供应商判断，编译目标是否来自苹果的设备
    #[inline]
    fn is_apple(&self) -> bool {
        self.vendor == "apple"
    }

    /// 通过供应商和操作系统判断，编译目标是否是 Apple MacOS 系统
    #[inline]
    fn is_apple_darwin(&self) -> bool {
        self.is_apple() && self.system == "darwin"
    }

    /// 通过操作系统判断，编译目标是否是 Android 系统
    #[inline]
    fn is_android(&self) -> bool {
        self.system.contains("android")
    }

    /// 通过操作系统和架构判断，编译目标是否 aarch64 平台上的 Android 系统
    #[inline]
    fn is_aarch64_android(&self) -> bool {
        self.is_android() && self.architecture == "aarch64"
    }

    /// 通过操作系统和架构判断，编译目标是否 armv7 平台上的 Android 系统
    #[inline]
    fn is_armv7_android(&self) -> bool {
        self.is_android() && self.architecture == "armv7"
    }

    /// 通过操作系统和架构判断，编译目标是否 x86_64 平台上的 Android 系统
    #[inline]
    fn is_x86_64_android(&self) -> bool {
        self.is_android() && self.architecture == "x86_64"
    }

    /// 通过操作系统和架构判断，编译目标是否 i686 平台上的 Android 系统
    #[inline]
    fn is_i686_android(&self) -> bool {
        self.is_android() && self.architecture == "i686"
    }

    /// 通过操作系统判断，编译目标是否是 Linux 系统
    #[inline]
    fn is_linux(&self) -> bool {
        self.system == "linux"
    }

    /// 通过操作系统判断，编译目标是否是 Windows 系统
    #[inline]
    fn is_windows(&self) -> bool {
        self.system == "windows"
    }

    /// 通过操作系统和 api 判断，编译目标是否是 Windows 系统，并且采用 MSVC 工具链编译
    #[inline]
    fn is_windows_msvc(&self) -> bool {
        self.is_windows() && self.environment == "msvc"
    }

    /// 通过 api 判断，采用 gnu 工具链编译
    #[inline]
    fn is_gnu(&self) -> bool {
        self.environment == "gnu"
    }
}

fn main() -> anyhow::Result<()> {
    // 定义 bindgen 生成代码文件的目录
    let binding_rs_out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("llama");
    // 编译产物所在的路径
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    // llama.cpp 源码路径
    let llama_src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("llama.cpp");

    // 监听可能变化的文件，当文件变化则重新构建
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    // 不监听一整个 llama.cpp 文件夹，这样会触发一些不必要的构建
    let entry_iter = walkdir::WalkDir::new(&llama_src_dir)
        .into_iter()
        .filter_entry(|e| {
            // 只需要非隐藏文件
            !e.file_name()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or_default()
        });
    for entry in entry_iter {
        let entry = entry.context("Failed to obtain file entry!")?;
        // 文件名中是否包含 CMakeLists.txt
        let contain_cmake = entry
            .file_name()
            .to_str()
            .is_some_and(|f| f.starts_with("CMakeLists.txt"));
        // 在 common 或者 ggml/src 或者 src 下
        let interest = entry.path().starts_with("common")
            | entry.path().starts_with("ggml/src")
            | entry.path().starts_with("src");
        let rebuild = contain_cmake | interest;
        if rebuild {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    // bindgen 配置
    let bindings = bindgen::Builder::default()
        // 指定生成 2024 版本的代码
        .rust_edition(RustEdition::Edition2024)
        .rust_target(RustTarget::nightly())
        .header("wrapper.h")
        // 指定 Clang 搜索头文件的路径
        .clang_arg(format!("-I{}", &llama_src_dir.join("include").display()))
        .clang_arg(format!(
            "-I{}",
            &llama_src_dir.join("ggml/include").display()
        ))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // 设置生成的代码中派生 PartialEq，即生成的 struct 上面有 #[derive(PartialEq)] 注解
        .derive_partialeq(true)
        .allowlist_function("ggml_.*")
        .allowlist_type("ggml_.*")
        .allowlist_function("llama_.*")
        .allowlist_type("llama_.*")
        // 不把 enum 附加到常量和 newType 变体
        .prepend_enum_name(false)
        .generate()
        .context("Failed to generate bindings")?;
    bindings
        // 指定生成的代码写入的文件，显式指定这个路径，目的为了 IDE 或者 LSP 可以分析到这个文件，进而提供更好的提示和代码完成
        .write_to_file(binding_rs_out_dir.join("bindings.rs"))
        .context("Failed to write bindings")?;

    // Cmake 配置，详情可以通过 llama.cpp 的 CMakeLists.txt 中了解
    let mut cmake_config = Config::new(&llama_src_dir);

    // 允许通过环境变量设置 CMake 在构建项目时的并行级别
    let parallel_level = env::var("BUILD_PARALLEL_LEVEL")
        .map_or(
            std::thread::available_parallelism()
                .context("Failed to obtain an estimate of the default amount of parallelism a program should use!")?
                .get(), |v| v.parse::<usize>().expect("Please provide a number!"),
        );
    unsafe {
        env::set_var("CMAKE_BUILD_PARALLEL_LEVEL", parallel_level.to_string());
    }

    // 编译期不开启测试
    cmake_config.define("LLAMA_BUILD_TESTS", "OFF");
    // 编译期不运行样例
    cmake_config.define("LLAMA_BUILD_EXAMPLES", "OFF");
    // 不编译 SERVER 组件
    cmake_config.define("LLAMA_BUILD_SERVER", "OFF");
    // 编译 LLaMA 模型时不生成共享库
    cmake_config.define("BUILD_SHARED_LIBS", "OFF");

    // 允许通过环境变量配置 llama.cpp 编译的 profile， 默认是 Release，并且监听这个环境变量
    let profile =
        env::var("LLAMA_CMAKE_BUILD_TYPE").map_or(CMakeBuildType::default(), String::into);
    println!("cargo:rerun-if-env-changed=LLAMA_CMAKE_BUILD_TYPE");
    cmake_config.profile(profile.as_str());

    // 允许通过环境变量配置 CMake 是否输出详细信息
    let verbose = env::var("CMAKE_VERBOSE").is_ok();
    cmake_config.very_verbose(verbose);

    // 允许通过环境变量配置 llama.cpp 模型在编译时是否使用静态运行时库（CRT），这个环境变量为布尔值 true 和 false，并且监听这个环境变量
    let static_crt = env::var("LLAMA_STATIC_CRT")
        .map(|v| v == "true")
        .unwrap_or(false);
    println!("cargo:rerun-if-env-changed=LLAMA_STATIC_CRT");
    // 设置是否静态运行时库
    cmake_config.static_crt(static_crt);

    // 获取目标三元组，针对不同的操作系统做不同的配置
    let target = TargetTriple::parse_from_env()?;
    println!("cargo:warning={target:?}");

    // 如果是苹果的系统，那么不编译 GGML_BLAS
    if target.is_apple() {
        cmake_config.define("GGML_BLAS", "OFF");
    }

    // 如果是 Windows 系统 msvc 工具链，并且 CMake 的 profile 不是 Debug，手动添加优化标识
    // 详细情况可看 https://github.com/rust-lang/cmake-rs/issues/240
    if target.is_windows_msvc() && profile != CMakeBuildType::Debug {
        for flag in &["/O2", "/DNDEBUG", "/Ob2"] {
            cmake_config.cflag(flag);
            cmake_config.cxxflag(flag);
        }
    }

    // 安卓系统配置
    if target.is_android() {
        // 需要通过环境变量获取到 NDK 的所在目录
        let android_ndk = env::var("ANDROID_NDK").context(
            "Please install Android NDK and ensure that ANDROID_NDK env variable is set",
        )?;
        println!("cargo::rerun-if-env-changed=ANDROID_NDK");

        // 指明 NDK 工具链
        cmake_config.define(
            "CMAKE_TOOLCHAIN_FILE",
            format!("{android_ndk}/build/cmake/android.toolchain.cmake"),
        );

        // 通过环境变量设置安卓版本
        let android_platform = env::var("ANDROID_PLATFORM").unwrap_or("android-28".to_owned());
        cmake_config.define("ANDROID_PLATFORM", android_platform);
        println!("cargo::rerun-if-env-changed=ANDROID_PLATFORM");

        // 需要指明安卓的架构
        if target.is_aarch64_android() || target.is_armv7_android() {
            cmake_config.cflag("-march=armv8.7a");
            cmake_config.cxxflag("-march=armv8.7a");
        } else if target.is_x86_64_android() {
            cmake_config.cflag("-march=x86-64");
            cmake_config.cxxflag("-march=x86-64");
        } else if target.is_i686_android() {
            cmake_config.cflag("-march=i686");
            cmake_config.cxxflag("-march=i686");
        } else {
            bail!("Unsupported Android target {target:?}")
        }

        // 不将  LLaMA 模型打包成一个单一文件
        cmake_config.define("GGML_LLAMAFILE", "OFF");

        // 开启 android 系统下共享 stdcxx 库
        if cfg!(feature = "android-shared-stdcxx") {
            println!("cargo:rustc-link-lib=dylib=stdc++");
            println!("cargo:rustc-link-lib=c++_shared");
        }
    }

    // 针对 feature 进行配置
    // 开启 vulkan，需要开启 cmake 的 vulkan 功能标志，并提供 vulkan 的 lib，供 rustc 链接
    if cfg!(feature = "vulkan") {
        cmake_config.define("GGML_VULKAN", "ON");
        if target.is_windows() {
            // 需要手动提供 vulkan 安装的目录
            let vulkan_path = env::var("VULKAN_SDK").context(
                "Please install Vulkan SDK and ensure that VULKAN_SDK env variable is set",
            )?;
            let vulkan_lib_path = Path::new(&vulkan_path).join("Lib");
            println!("cargo:rustc-link-search={}", vulkan_lib_path.display());
            println!("cargo:rustc-link-lib=vulkan-1");
        } else if target.is_linux() {
            println!("cargo:rustc-link-lib=vulkan");
        }
    }

    // 开启 cuda 功能则需要开启 cmake 的 cuda 功能标志
    if cfg!(feature = "cuda") {
        cmake_config.define("GGML_CUDA", "ON");
        if cfg!(feature = "cuda-no-vmm") {
            cmake_config.define("GGML_CUDA_NO_VMM", "ON");
        }
    }

    // 开启 openmp 功能（OpenMP主要用于多线程并行计算）
    // openmp 在安卓上性能提升不明显，安卓平台优先使用 vulkan 编译
    if cfg!(feature = "openmp") && !target.is_android() {
        cmake_config.define("GGML_OPENMP", "ON");
    } else {
        cmake_config.define("GGML_OPENMP", "OFF");
    }

    let build_dir = cmake_config.build();
    let build_info_src = llama_src_dir.join("common/build-info.cpp");
    let build_info_target = build_dir.join("build-info.cpp");
    rename(&build_info_src, &build_info_target).unwrap_or_else(|rename_error| {
        copy(&build_info_src, &build_info_target).unwrap_or_else(|copy_error| {
            panic!("Failed to rename {build_info_src:?} to {build_info_target:?}. Move failed with {rename_error:?} and copy failed with {copy_error:?}");
        });
        remove_file(&build_info_src).unwrap_or_else(|remove_error| {
            panic!("Failed to delete {build_info_src:?} after copying to {build_info_target:?}: {remove_error:?} (move failed because {rename_error:?})");
        });
    });

    // 链接阶段，提供需要链接的 lib 目录
    println!("cargo:rustc-link-search={}", out_dir.join("lib").display());
    println!(
        "cargo:rustc-link-search={}",
        out_dir.join("lib64").display()
    );
    println!("cargo:rustc-link-search={}", build_dir.display());

    if cfg!(feature = "cuda") {
        // 寻找 cuda 安装的路径
        for lib_dir in find_cuda_helper::find_cuda_lib_dirs() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
        }

        println!("cargo:rustc-link-lib=static=cudart_static");

        if target.is_windows() {
            println!("cargo:rustc-link-lib=static=cublas");
            println!("cargo:rustc-link-lib=static=cublasLt");
        } else {
            println!("cargo:rustc-link-lib=static=cublas_static");
            println!("cargo:rustc-link-lib=static=cublasLt_static");
        }

        if !cfg!(feature = "cuda-no-vmm") {
            println!("cargo:rustc-link-lib=cuda");
        }

        println!("cargo:rustc-link-lib=static=culibos");
    }

    let llama_libs_kind = "static";
    let llama_libs = extract_lib_names(&out_dir, &target)?;
    assert_ne!(llama_libs.len(), 0);

    for lib in llama_libs {
        // 静态链接到这些编译后的库产物中
        println!("cargo:rustc-link-lib={llama_libs_kind}={lib}",);
    }

    // OpenMP
    if cfg!(feature = "openmp") && target.is_gnu() {
        println!("cargo:rustc-link-lib=gomp");
    }

    // 链接 c++ runtime
    if target.is_linux() {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    } else if target.is_apple() {
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalKit");
        println!("cargo:rustc-link-lib=framework=Accelerate");
        println!("cargo:rustc-link-lib=c++");

        if target.is_apple_darwin() {
            if let Ok(path) = macos_link_search_path() {
                println!("cargo:rustc-link-lib=clang_rt.osx");
                println!("cargo:rustc-link-search={}", path);
            }
        }
    }

    Ok(())
}

fn extract_lib_names(out_dir: &Path, target: &TargetTriple) -> anyhow::Result<Vec<String>> {
    let lib_pattern = if target.is_windows() { "*.lib" } else { "*.a" };
    let libs_dir = out_dir.join("lib*");
    let pattern = libs_dir.join(lib_pattern);
    println!("cargo:warning=Extract libs {}", pattern.display());

    let mut lib_names: Vec<String> = Vec::new();
    // 通过指定的 pattern 找到所有编译生成的库产物
    for entry in glob(pattern.to_str().unwrap())? {
        match entry {
            Ok(path) => {
                let stem = path.file_stem().unwrap();
                let stem_str = stem.to_str().unwrap();
                // 移除 lib 前缀
                let lib_name = if stem_str.starts_with("lib") {
                    stem_str.strip_prefix("lib").unwrap_or(stem_str)
                } else {
                    if path.extension() == Some(std::ffi::OsStr::new("a")) {
                        let target = path.parent().unwrap().join(format!("lib{}.a", stem_str));
                        rename(&path, &target).context("Failed to rename lib")?;
                    }
                    stem_str
                };
                lib_names.push(lib_name.to_string());
            }
            Err(e) => return Err(anyhow!("Match failure, error was {e:?}")),
        }
    }
    Ok(lib_names)
}

fn macos_link_search_path() -> anyhow::Result<String> {
    let output = Command::new("clang").arg("--print-search-dirs").output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to run 'clang --print-search-dirs', continuing without a link search path"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("libraries: =") {
            let path = line.split('=').nth(1).unwrap();
            return Ok(format!("{path}/lib/darwin"));
        }
    }

    Err(anyhow!(
        "failed to determine link search path, continuing without it"
    ))
}
