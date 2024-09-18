use crate::utils::*;
use crate::makepad_shell::*;

fn create_deveco_project(args : &[String]) -> Result<(), String> {
    let build_crate = get_build_crate_from_args(args)?;
    let underscore_build_crate = build_crate.replace('-', "_");

    let prj_path = std::env::current_dir().unwrap().join(format!("target/makepad-open-haromony/{underscore_build_crate}"));
    let tpl_path = std::env::current_dir().unwrap().join("tools/open_harmony/deveco");
    let _= rmdir(&prj_path);
    mkdir(&prj_path)?;
    cp_all(&tpl_path, &prj_path, false)?;
    Ok(())
}

pub fn build(deveco_home: &Option<String>, args: &[String]) ->  Result<(), String> {
    if deveco_home.is_none() {
        return Err("--deveco-home is not specified".to_owned());
    }
    create_deveco_project(args)?;
    Ok(())
}