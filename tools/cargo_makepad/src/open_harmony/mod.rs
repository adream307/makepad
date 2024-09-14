mod compile;
mod sdk;
use compile::*;

#[derive(Clone, Copy, PartialEq)]
pub enum HostOs {
    WindowsX64,
    MacosX64,
    MacosAarch64,
    LinuxX64,
    Unsupported
}

#[allow(non_camel_case_types)]
pub enum OpenHarmonyTarget {
    aarch64,
    x86_64,
    armv7
}

impl OpenHarmonyTarget {
    fn toolchain(&self) ->&'static str{
        match self {
            Self::aarch64 => "aarch64-unknown-linux-ohos",
            Self::armv7 => "armv7-unknown-linux-ohos",
            Self::x86_64 => "x86_64-unknown-linux-ohos"

        }
    }
}

pub fn handle_open_harmony(args: &[String]) -> Result<(), String> {
    if args.len() < 1 {
        return Err(format!("not enough args"))
    }
    match args[0].as_ref() {
        "toolchain-install" | "install-toolchain" => {
            let toolchains = vec![OpenHarmonyTarget::aarch64];
            sdk::rustup_toolchain_install(&toolchains)
        }
        _ => Err(format!("{} is not a valid command or option", args[0]))

    }
}

