use {
    std::{
        io::prelude::*,
        fs::File,
        rc::Rc,
    },
    crate::{
        cx::{Cx},
    }
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum EventFlow{
    Poll,
    Wait,
    Exit
}

// lets start a websocket thread


impl Cx {
    
    pub fn native_load_dependencies(&mut self){
        for (path,dep) in &mut self.dependencies{
            crate::log!("===================== dependencies old:{}",path);
            let path1 = path.replace("makepad_widgets/resources/icons/", "");
            let path2 = path1.replace("makepad_widgets/resources/", "");
            crate::log!("===================== dependencies new:{}",path2);
            if let Ok(mut file_handle) = File::open(&path2) {
                let mut buffer = Vec::<u8>::new();
                if file_handle.read_to_end(&mut buffer).is_ok() {
                    dep.data = Some(Ok(Rc::new(buffer)));
                }
                else{
                    dep.data = Some(Err("read_to_end failed".to_string()));
                }
            }
            else{
                println!("Could not load resource {}", path2);
                dep.data = Some(Err("File open failed".to_string()));
            }
        }
    }
}
