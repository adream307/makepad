use std::fs;
use std::path::Path;
use crate::utils::*;
use crate::makepad_shell::*;

fn is_prj_dir_used(path: &String) -> bool {
    let path = Path::new(path);
    if let Ok(meta) = fs::metadata(path) {
        if meta.is_file() || meta.is_symlink() {
            true;
        }
        //is dir
        if let Ok(prj_meta) = fs::metadata(path.join("build-profile.json5")) {
            if !prj_meta.is_file() {
                return true;
            }
        }
        false
    }else {
        false
    }
}

fn create_deveco_project(path: &String) -> bool {
    let prj_path = Path::new(path);
    if let Ok(meta) =  fs::metadata(prj_path) {
        if meta.is_dir() {
            return true;
        }
    }
    match fs::create_dir_all(path) {
        Ok(_) => {
            let cwd = std::env::current_dir().unwrap();
            let src_path =  cwd.join("tools/open_harmony/deveco");
            if let Err(_) = cp_all(&src_path, &prj_path, false) {
                return false;
            }
            true
        },
        Err(_) => false
    }
}

pub fn build(deveco_home: &Option<String>, prj_path: &Option<String>) ->  Result<(), String> {
    if deveco_home.is_none() {
        return Err("--deveco-home is not specified".to_owned());
    }
    if prj_path.is_none() {
        return Err("--project-path is not specified".to_owned());
    }
    let tpl_path = std::env::current_dir().unwrap().join("tools/open_harmony/deveco");

    let path = Path::new(prj_path.as_ref().unwrap());
    if let Ok(path_meta) = fs::metadata(path) {
        if path_meta.is_file() || path_meta.is_symlink() {
            return Err(format!("project-path = {} is used by others",prj_path.as_ref().unwrap()));
        }
        if let Ok(build_meta) = fs::metadata(path.join("build-profile.json5")) {
            if !build_meta.is_file() {
                return Err(format!("project-path = {} is used by others",prj_path.as_ref().unwrap()));
            }
        } else {
            return Err(format!("project-path = {} is used by others",prj_path.as_ref().unwrap()));
        }
    }else{
        let _ = fs::create_dir_all(prj_path.as_ref().unwrap());
        cp_all(&tpl_path, path, false)?;
    }

    Ok(())
}