use crate::open_harmony::OpenHarmonyTarget;
use crate::open_harmony::HostOs;
use crate::utils::*;
use crate::makepad_shell::*;
use std::path::Path;
use std::path::PathBuf;

fn get_sdk_path(deveco_home : &Path, host_os: &HostOs) -> Result<PathBuf, String> {
    match host_os {
        HostOs::LinuxX64 => {
            let sdk_path = deveco_home.join("sdk/HarmonyOS-NEXT-DB2/openharmony/native");
            Ok(sdk_path)
        },
        _ => panic!()
    }
}

fn rust_build(deveco_home: &Option<String>, host_os: HostOs, args: &[String], targets:&[OpenHarmonyTarget]) -> Result<(), String> {
    let deveco_home = Path::new(deveco_home.as_ref().unwrap());
    let cwd = std::env::current_dir().unwrap();
    let sdk_path = get_sdk_path(deveco_home, &host_os)?;

    let bin_path = | file_name: &str, extension:& str | match host_os {
        HostOs::LinuxX64 => String::from(file_name),
        HostOs::WindowsX64 => format!("{file_name}.{extension}"),
        HostOs::MacosX64 => String::from(file_name),
        _ => panic!()
    };

    let full_clang_path = sdk_path.join(bin_path("llvm/bin/aarch64-unknown-linux-ohos-clang","cmd"));
    let full_clangpp_path = sdk_path.join(bin_path("llvm/bin/aarch64-unknown-linux-ohos-clang","cmd"));
    let full_llvm_ar_path = sdk_path.join(bin_path("llvm/bin/llvm-ar","exe"));
    let full_llvm_ranlib_path = sdk_path.join(bin_path("llvm/bin/llvm-ranlib","exe"));
    for target in targets {
        let toolchain = target.toolchain();
        let target_opt = format!("--target={toolchain}");
        let toolchain = toolchain.replace('-',"_");

        let base_args = &[
            "run",
            "nightly",
            "cargo",
            "rustc",
            "--lib",
            "--crate-type=cdylib",
            &target_opt
        ];
        let mut args_out = Vec::new();
        args_out.extend_from_slice(base_args);
        for arg in args {
            args_out.push(arg);
        }
        let makepad_env = std::env::var("MAKEPAD").unwrap_or("lines".to_string());
        shell_env(
            &[
                (&format!("CC_{toolchain}"),     full_clang_path.to_str().unwrap()),
                (&format!("CXX_{toolchain}"),    full_clangpp_path.to_str().unwrap()),
                (&format!("AR_{toolchain}"),     full_llvm_ar_path.to_str().unwrap()),
                (&format!("RANLIB_{toolchain}"), full_llvm_ranlib_path.to_str().unwrap()),
                (&format!("CARGO_TARGET_{}_LINKER",toolchain.to_uppercase()), full_clang_path.to_str().unwrap()),
                ("MAKEPAD", &makepad_env),
            ],
            &cwd,
            "rustup",
            &args_out)?;
    }
    Ok(())
}

fn create_deveco_project(args : &[String], _targets :&[OpenHarmonyTarget]) -> Result<(), String> {
    let build_crate = get_build_crate_from_args(args)?;
    let underscore_build_crate = build_crate.replace('-', "_");

    let prj_path = std::env::current_dir().unwrap().join(format!("target/makepad-open-haromony/{underscore_build_crate}"));
    let raw_file = prj_path.join("entry/src/main/resources/rawfile");
    let tpl_path = std::env::current_dir().unwrap().join("tools/open_harmony/deveco");
    let _= rmdir(&prj_path);
    mkdir(&prj_path)?;
    cp_all(&tpl_path, &prj_path, false)?;
    mkdir(&raw_file)?;

    let build_crate_dir = get_crate_dir(build_crate)?;
    let local_resources_path = build_crate_dir.join("resources");
    if local_resources_path.is_dir() {
        let dst_dir = raw_file.join(format!("makepad/{underscore_build_crate}/resources"));
        mkdir(&dst_dir)?;
        cp_all(&local_resources_path, &dst_dir, false)?;
    }
    Ok(())
}

pub fn build(deveco_home: &Option<String>, args: &[String], host_os: HostOs, targets :&[OpenHarmonyTarget]) ->  Result<(), String> {
    if deveco_home.is_none() {
        return Err("--deveco-home is not specified".to_owned());
    }
    rust_build(&deveco_home, host_os, &args, &targets)?;
    create_deveco_project(args, &targets)?;
    Ok(())
}