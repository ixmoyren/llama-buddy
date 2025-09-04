use anyhow::{Context, bail};
use bindgen::{Bindings, RustEdition};
use cmake::Config;
use glob::glob;
use std::{
    cmp::PartialEq,
    env,
    fs::rename,
    path::{Path, PathBuf},
    process::Command,
};
use sys_extra::target::TargetTriple;

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

fn main() -> anyhow::Result<()> {
    // 定义 bindgen 生成代码文件的目录
    let binding_rs_out_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("llama");
    // 编译产物所在的路径
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    // llama.cpp 源码路径
    let llama_src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("llama.cpp");

    // 监听可能变化的文件，当文件变化则重新构建
    cargo_rerun_if_file_changed(&llama_src_dir)?;

    // bindgen 配置
    let bindings = make_bindgen(&llama_src_dir)?;
    // 指定生成的代码写入的文件，显式指定这个路径，目的为了 IDE 或者 LSP 可以分析到这个文件，进而提供更好的提示和代码完成
    bindings
        .write_to_file(binding_rs_out_dir.join("bindings.rs"))
        .context("Failed to write bindings")?;

    // 获取目标三元组，针对不同的操作系统做不同的配置
    let target = TargetTriple::parse_from_env()?;
    println!("cargo:warning={target:?}");

    // Cmake 配置，详情可以通过 llama.cpp 的 CMakeLists.txt 中了解
    let mut cmake_config = make_cmake_config(&llama_src_dir, &target)?;

    // 如果是苹果的系统，那么不编译 GGML_BLAS
    if target.is_apple() {
        cmake_config.define("GGML_BLAS", "OFF");
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

        // 不将 LLAMA.CPP 打包成一个单一文件
        cmake_config.define("GGML_LLAMAFILE", "OFF");

        // 开启 android 系统下共享 stdcxx 库
        if cfg!(feature = "android-shared-stdcxx") {
            println!("cargo:rustc-link-lib=dylib=stdc++");
            println!("cargo:rustc-link-lib=c++_shared");
        }
    }

    // 针对 feature 进行配置
    // 开启 vulkan，需要开启 cmake 的 vulkan 功能标志，并提供 vulkan 的 lib，供 rustc 链接
    #[cfg(feature = "vulkan")]
    open_vulkan_backend(&mut cmake_config, &target)?;

    // 开启 cuda 功能则需要开启 cmake 的 cuda 功能标志
    #[cfg(any(feature = "cuda", feature = "cuda-no-vmm"))]
    open_cuda_backend(&mut cmake_config)?;

    // 开启 openmp 功能（OpenMP主要用于多线程并行计算）
    // openmp 在安卓上性能提升不明显，安卓平台优先使用 vulkan 编译
    #[cfg(feature = "openmp")]
    open_openmp_backend(&mut cmake_config, &target)?;

    let build_dir = cmake_config.build();
    // 链接阶段，提供需要链接的 lib 目录
    cargo_rustc_link_llama_cpp_lib(&out_dir, &build_dir, &target)?;

    // 链接 CPP 标准库
    cargo_rustc_link_cpp_lib(&target)?;

    #[cfg(any(feature = "cuda", feature = "cuda-no-vmm"))]
    cargo_rustc_link_cuda_lib(&target)?;

    #[cfg(feature = "openmp")]
    cargo_rustc_link_openmp_lib(&target)?;

    Ok(())
}

/// 设置 rustc 链接到 openmp 的动态库
#[cfg(feature = "openmp")]
fn cargo_rustc_link_openmp_lib(target: &TargetTriple) -> anyhow::Result<()> {
    if target.is_gnu() {
        println!("cargo:rustc-link-lib=gomp");
    }
    Ok(())
}

/// 设置 rustc 链接到 CUDA 的动态库
#[cfg(any(feature = "cuda", feature = "cuda-no-vmm"))]
fn cargo_rustc_link_cuda_lib(target: &TargetTriple) -> anyhow::Result<()> {
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
    Ok(())
}

/// 设置 rustc 链接 C++ 标准库
fn cargo_rustc_link_cpp_lib(target: &TargetTriple) -> anyhow::Result<()> {
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
            let path = macos_link_search_path()?;
            println!("cargo:rustc-link-lib=clang_rt.osx");
            println!("cargo:rustc-link-search={path}");
        }
    }

    Ok(())
}

fn macos_link_search_path() -> anyhow::Result<String> {
    let output = Command::new("clang").arg("--print-search-dirs").output()?;
    if !output.status.success() {
        bail!("failed to run 'clang --print-search-dirs', continuing without a link search path")
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("libraries: =") {
            let path = line.split('=').nth(1).unwrap();
            return Ok(format!("{path}/lib/darwin"));
        }
    }

    bail!("failed to determine the link search path, continuing without it")
}

/// 设置 rustc 链接到 LLAMA.CPP 的编译产物
fn cargo_rustc_link_llama_cpp_lib(
    out: &Path,
    build: &Path,
    target: &TargetTriple,
) -> anyhow::Result<()> {
    println!("cargo:rustc-link-search={}", out.join("lib").display());
    println!("cargo:rustc-link-search={}", out.join("lib64").display());
    println!("cargo:rustc-link-search={}", build.display());
    let llama_libs_kind = "static";
    let llama_libs = extract_lib_names(out, target)?;
    assert_ne!(llama_libs.len(), 0);

    for lib in llama_libs {
        // 静态链接到这些编译后的库产物中
        println!("cargo:rustc-link-lib={llama_libs_kind}={lib}",);
    }

    Ok(())
}

/// 调整 lib 的名称
fn extract_lib_names(out_dir: &Path, target: &TargetTriple) -> anyhow::Result<Vec<String>> {
    let lib_pattern = if target.is_windows() { "*.lib" } else { "*.a" };
    let libs_dir = out_dir.join("lib*");
    let pattern = libs_dir.join(lib_pattern);
    println!("cargo:warning=Extract libs {}", pattern.display());

    let Some(pattern) = pattern.to_str() else {
        bail!("Failed to get lib pattern str")
    };

    let mut lib_names: Vec<String> = Vec::new();
    // 文件的后缀名为 a
    let path_extension_a = std::ffi::OsStr::new("a");
    // 通过指定的 pattern 找到所有编译生成的库产物
    for entry in glob(pattern)? {
        match entry {
            Ok(path) => {
                if let Some(stem) = path.file_stem()
                    && let Some(stem_str) = stem.to_str()
                {
                    // 移除 lib 前缀
                    let lib_name = if stem_str.starts_with("lib") {
                        stem_str.strip_prefix("lib").unwrap_or(stem_str)
                    } else {
                        let Some(extension) = path.extension() else {
                            bail!("Failed to get lib file path extension, {path:?}")
                        };
                        if extension == path_extension_a
                            && let Some(parent) = path.parent()
                        {
                            let target = parent.join(format!("lib{stem_str}.a"));
                            rename(&path, &target).context("Failed to rename lib")?;
                        }
                        stem_str
                    };
                    lib_names.push(lib_name.to_string());
                } else {
                    bail!("Failed to get lib file stem str, {path:?}")
                }
            }
            Err(e) => bail!("Match failure, error was {e:?}"),
        }
    }
    Ok(lib_names)
}

/// 针对 openmp 进行配置
#[cfg(feature = "openmp")]
fn open_openmp_backend(cmake_config: &mut Config, target: &TargetTriple) -> anyhow::Result<()> {
    // openmp 在安卓上性能提升不明显，安卓平台优先使用 vulkan 编译
    if !target.is_android() {
        cmake_config.define("GGML_OPENMP", "ON");
    } else {
        cmake_config.define("GGML_OPENMP", "OFF");
    }
    Ok(())
}

/// 针对 CUDA 进行配置
#[cfg(any(feature = "cuda", feature = "cuda-no-vmm"))]
fn open_cuda_backend(cmake_config: &mut Config) -> anyhow::Result<()> {
    cmake_config.define("GGML_CUDA", "ON");
    if cfg!(feature = "cuda-no-vmm") {
        cmake_config.define("GGML_CUDA_NO_VMM", "ON");
    }
    Ok(())
}

/// 针对 vulkan 进行配置
#[cfg(feature = "vulkan")]
fn open_vulkan_backend(cmake_config: &mut Config, target: &TargetTriple) -> anyhow::Result<()> {
    cmake_config.define("GGML_VULKAN", "ON");
    if target.is_windows() {
        // 需要手动提供 vulkan 安装的目录
        let vulkan_path = env::var("VULKAN_SDK")
            .context("Please install Vulkan SDK and ensure that VULKAN_SDK env variable is set")?;
        let vulkan_lib_path = Path::new(&vulkan_path).join("Lib");
        println!("cargo:rustc-link-search={}", vulkan_lib_path.display());
        println!("cargo:rustc-link-lib=vulkan-1");
        // 详情 https://github.com/utilityai/llama-cpp-rs/pull/767
        unsafe {
            env::set_var("TrackFileAccess", "false");
        }
        cmake_config.cflag("/FS");
        cmake_config.cxxflag("/FS");
    } else if target.is_linux() {
        println!("cargo:rustc-link-lib=vulkan");
    }
    Ok(())
}

/// 构建 cmake 的配置
///
/// 允许通过环境变量设置 CMake 构建项目时的并行级别
///
/// 不编译 LLAMA.CPP 的测试库
///
/// 不编译 LLAMA.CPP 的示例库
///
/// 不编译 LLAMA.CPP 的 SERVER 模块
///
/// 不编译 LLAMA.CPP 的 TOOL 模块
///
/// 不编译 LLAMA.CPP 的 CURL 模块
///
/// 不生成静态共享库
///
/// 允许通过环境变量配置 LLAMA.CPP 编译的 profile，默认是 Release
///
/// 允许通过环境变量配置 CMake 是否输出详细信息，默认不输出详细信息
///
/// 允许通过环境变量配置 LLAMA.CPP 在编译时是否使用静态运行时库，默认不使用
///
/// 如果编译工具是 Windows 系统 msvc ，并且 CMake 的 profile 不是 Debug，手动添加优化标识
fn make_cmake_config(llama_src: &Path, target: &TargetTriple) -> anyhow::Result<Config> {
    let mut cmake_config = Config::new(llama_src);

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
    // 不编译 TOOL 组件
    cmake_config.define("LLAMA_BUILD_TOOLS", "OFF");
    // 不编译 CURL 组件
    cmake_config.define("LLAMA_CURL", "OFF");

    // 不生成共享库
    cmake_config.define("BUILD_SHARED_LIBS", "OFF");

    // 允许通过环境变量配置 llama.cpp 编译的 profile， 默认是 Release，并且监听这个环境变量
    let profile =
        env::var("LLAMA_CMAKE_BUILD_TYPE").map_or(CMakeBuildType::default(), String::into);
    println!("cargo:rerun-if-env-changed=LLAMA_CMAKE_BUILD_TYPE");
    cmake_config.profile(profile.as_str());

    // 允许通过环境变量配置 CMake 是否输出详细信息
    let verbose = env::var("CMAKE_VERBOSE").is_ok();
    cmake_config.very_verbose(verbose);

    // 允许通过环境变量配置 llama.cpp 在编译时是否使用静态运行时库（CRT），这个环境变量为布尔值 true 和 false，并且监听这个环境变量
    let static_crt = env::var("LLAMA_STATIC_CRT")
        .map(|v| v == "true")
        .unwrap_or(false);
    println!("cargo:rerun-if-env-changed=LLAMA_STATIC_CRT");
    // 设置是否静态运行时库
    cmake_config.static_crt(static_crt);

    // 如果是 Windows 系统 msvc 工具链，并且 CMake 的 profile 不是 Debug，手动添加优化标识
    // 详细情况可看 https://github.com/rust-lang/cmake-rs/issues/240
    if target.is_windows_msvc() && profile != CMakeBuildType::Debug {
        for flag in &["/O2", "/DNDEBUG", "/Ob2"] {
            cmake_config.cflag(flag);
            cmake_config.cxxflag(flag);
        }
    }

    Ok(cmake_config)
}

/// 构建 Binding
///
/// 指定 bindgen 相关配置，生成符合 2024 版本的代码
///
/// 指定头文件的包装文件
///
/// 指定生成的代码中的结构体派生 PartialEq
///
/// 指定需要关注的函数和类型
fn make_bindgen(llama_src: &Path) -> anyhow::Result<Bindings> {
    let bindings = bindgen::Builder::default()
        // 指定生成 2024 版本的代码
        .rust_edition(RustEdition::Edition2024)
        .header("wrapper.h")
        // 指定 Clang 搜索头文件的路径
        .clang_arg(format!("-I{}", llama_src.join("include").display()))
        .clang_arg(format!("-I{}", llama_src.join("ggml/include").display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .use_core()
        .allowlist_function("ggml_.*")
        .allowlist_type("ggml_.*")
        .allowlist_function("llama_.*")
        .allowlist_type("llama_.*")
        // 不把 enum 附加到常量和 newType 变体
        .prepend_enum_name(false)
        .generate()
        .context("Failed to generate bindings")?;
    Ok(bindings)
}

/// 监听文件，如果有变化，则重新运行 cargo
fn cargo_rerun_if_file_changed(llama_src: &Path) -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    // 不监听一整个 llama.cpp 文件夹，这样会触发一些不必要的构建
    let entry_iter = walkdir::WalkDir::new(llama_src)
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
        // 判断当前文件的名称是否包含 CMakeLists.txt，包含则需要监听
        let contain_cmake = entry
            .file_name()
            .to_str()
            .is_some_and(|f| f.starts_with("CMakeLists.txt"));
        // 判断当前文件是否在 common 或者 ggml/src 或者 src 下，是则需要监听
        let interest = entry.path().starts_with("common")
            | entry.path().starts_with("ggml/src")
            | entry.path().starts_with("src");
        let rebuild = contain_cmake | interest;
        if rebuild {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    Ok(())
}
