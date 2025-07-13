use std::process::{Command, Output};

fn main() {
    // 监听可能变化的文件，当文件变化则重新构建
    println!("cargo:rerun-if-changed=build.rs");
    // 更新前端依赖
    let output = Command::new("pnpm")
        .arg("update")
        .output()
        .expect("Failed to execute pnpm update");
    print_output(output);
    // 安装前端依赖
    let output = Command::new("pnpm")
        .arg("install")
        .output()
        .expect("Failed to execute pnpm install");
    print_output(output);
    // 压缩 scala-api-reference.js
    let output = Command::new("pnpm")
        .arg("run")
        .arg("build:compress")
        .output()
        .expect("Failed to execute pnpm run build:compress");
    print_output(output);
}

fn print_output(output: Output) {
    if output.status.success() {
        println!("cargo:warning={}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("cargo:error={}", String::from_utf8_lossy(&output.stderr));
    }
}
