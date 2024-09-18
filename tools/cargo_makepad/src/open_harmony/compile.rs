use std::fs;
use std::path::Path;
use crate::utils::*;
use crate::makepad_shell::*;

pub fn build(deveco_home: &Option<String>, prj_path: &Option<String>, args: &[String]) ->  Result<(), String> {
    if deveco_home.is_none() {
        return Err("--deveco-home is not specified".to_owned());
    }
    if prj_path.is_none() {
        return Err("--project-path is not specified".to_owned());
    }
    let build_crate = get_build_crate_from_args(args)?;

    println!("open harmony: deveco_home={}",deveco_home.as_ref().unwrap());
    println!("open harmony: project_path={}",prj_path.as_ref().unwrap());
    println!("open harmony: build_crate={}",build_crate);
    let tpl_path = std::env::current_dir().unwrap().join("tools/open_harmony/deveco");
    let underscore_build_crate = build_crate.replace('-', "_");

    let path = Path::new(prj_path.as_ref().unwrap()).join(underscore_build_crate);
    if let Ok(path_meta) = fs::metadata(&path) {
        if path_meta.is_file() || path_meta.is_symlink() {
            return Err(format!("project-path = {} is used by others",prj_path.as_ref().unwrap()));
        }
        if let Ok(build_meta) = fs::metadata(path.clone().join("build-profile.json5")) {
            if !build_meta.is_file() {
                return Err(format!("project-path = {} is used by others",prj_path.as_ref().unwrap()));
            }
        } else {
            return Err(format!("project-path = {} is used by others",prj_path.as_ref().unwrap()));
        }
    }else{
        let _ = fs::create_dir_all(prj_path.as_ref().unwrap());
        cp_all(&tpl_path, &path, false)?;
    }

    Ok(())
}