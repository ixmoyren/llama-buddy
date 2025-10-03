use std::{
    fs::File,
    io::BufReader,
    process::{Command, Output},
};

fn main() {
    // 监听可能变化的文件，当文件变化则重新构建
    println!("cargo:rerun-if-changed=build.rs");
    // 如果静态文件所在的目录不存在，那么直接安装
    let static_dir = std::env::current_dir()
        .expect("Failed to get the current directory")
        .join("static");
    if !static_dir.exists() {
        println!("cargo:warning=The static dir of scalar does not exist.");
        install_and_compress();
        return;
    }
    // 如果静态文件不存在，那么直接安装
    let static_js = static_dir.join("scalar-api-reference.js");
    if !static_js.exists() {
        println!("cargo:warning=The static file of scalar does not exist.");
        install_and_compress();
        return;
    }
    // 通过 pnpm 获取到 @scalar/api-reference 最新的版本
    let Some(scalar_version) = get_scalar_version_by_pnpm() else {
        // 直接退出
        return;
    };
    // 通过 serde_json 读取 package.json
    let Some(scalar_version_another) = get_scalar_version_from_package_json() else {
        return;
    };
    if scalar_version_another != scalar_version {
        println!("cargo:warning=The scalar has a new version.");
        install_and_compress();
    }
}

fn get_scalar_version_from_package_json() -> Option<String> {
    let package_json = std::env::current_dir()
        .expect("Failed to get the current directory")
        .join("package.json");
    let package_json_file = File::open(package_json).expect("Failed to open package.json");
    let reader = BufReader::new(package_json_file);
    let json = serde_json::from_reader::<_, serde_json::Value>(reader)
        .expect("Failed to parse package.json");
    if let Some(dev_dependencies) = json.get("devDependencies")
        && let Some(scalar) = dev_dependencies.get("@scalar/api-reference")
        && let Some(version) = scalar.as_str()
    {
        Some(version.replace("^", ""))
    } else {
        println!("cargo:error=Failed to get scalar version from package.json");
        None
    }
}

fn get_scalar_version_by_pnpm() -> Option<String> {
    // 通过 pnpm 获取到 @scalar/api-reference 最新的版本
    let output = Command::new("pnpm")
        .arg("info")
        .arg("@scalar/api-reference")
        .arg("version")
        .output()
        .expect("Failed to execute pnpm update");
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        println!(
            "cargo:error=Failed to get scalar version from npm, err is {}, {}",
            String::from_utf8_lossy(&output.stderr),
            &output.status
        );
        None
    }
}

fn install_and_compress() {
    // 更新前端依赖
    let output = Command::new("pnpm")
        .arg("update")
        .output()
        .expect("Failed to execute pnpm update");
    print_output("pnpm update", output);
    // 安装前端依赖
    let output = Command::new("pnpm")
        .arg("install")
        .output()
        .expect("Failed to execute pnpm install");
    print_output("pnpm install", output);
    // 压缩 scala-api-reference.js
    let output = Command::new("pnpm")
        .arg("run")
        .arg("build:compress")
        .output()
        .expect("Failed to execute pnpm run build:compress");
    print_output("pnpm run build:compress", output);
}

fn print_output(
    msg: &str,
    Output {
        status,
        stdout,
        stderr,
    }: Output,
) {
    if status.success() {
        let out = if stdout.is_empty() {
            "".to_owned()
        } else {
            format!(", out is {}", String::from_utf8_lossy(&stdout))
        };
        println!("cargo:warning={msg}{out}");
    } else {
        let err = if stderr.is_empty() {
            format!(", {status}")
        } else {
            format!(", err is {}, {status}", String::from_utf8_lossy(&stderr))
        };
        println!("cargo:error={msg}{err}",);
    }
}
