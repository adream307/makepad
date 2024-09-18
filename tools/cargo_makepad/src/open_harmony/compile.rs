use std::fs;
use crate::utils::*;
use crate::makepad_shell::*;

pub fn build(deveco_home: &Option<String>, args: &[String]) ->  Result<(), String> {
    if deveco_home.is_none() {
        return Err("--deveco-home is not specified".to_owned());
    }
    let build_crate = get_build_crate_from_args(args)?;
    let underscore_build_crate = build_crate.replace('-', "_");

    let prj_path = std::env::current_dir().unwrap().join(format!("target/makepad-open-haromony/{underscore_build_crate}"));

    println!("open harmony: deveco_home={}",deveco_home.as_ref().unwrap());
    println!("open harmony: project_path={}",prj_path.to_str().unwrap());
    let tpl_path = std::env::current_dir().unwrap().join("tools/open_harmony/deveco");
    let _ = rmdir(&prj_path);
    let _ = fs::create_dir_all(&prj_path);
    let _ = cp_all(&tpl_path, &prj_path, false);
    

    Ok(())
}