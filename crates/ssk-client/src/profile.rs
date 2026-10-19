use serde::{Serialize,Deserialize};
use std::path::PathBuf;
#[derive(Serialize,Deserialize)]
#[serde(default)]
pub struct Profile {
    pub records:[u64;3],pub volume:f32,pub shake:f32,pub flashes:bool,
    pub ui_scale:f32,pub fullscreen:bool,pub width:i32,pub height:i32,pub class:usize,
}
impl Default for Profile {
    fn default()->Self {Self {records:[0;3],volume:0.6,shake:0.7,flashes:true,ui_scale:1.,fullscreen:false,width:1440,height:900,class:1}}
}
impl Profile {
    pub fn path()->PathBuf {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(||PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")).join("ssk/profile.json")
    }
    pub fn load()->Self {
        let mut profile:Self=std::fs::read(Self::path()).ok().and_then(|b|serde_json::from_slice(&b).ok()).unwrap_or_default();
        profile.volume=profile.volume.clamp(0.,1.);profile.shake=profile.shake.clamp(0.,1.);profile.ui_scale=profile.ui_scale.clamp(0.75,1.25);
        profile.width=profile.width.clamp(800,7680);profile.height=profile.height.clamp(600,4320);profile.class=profile.class.min(2);profile
    }
    pub fn save(&self)->std::io::Result<()> {
        let path=Self::path();std::fs::create_dir_all(path.parent().unwrap())?;
        let temp=path.with_extension("tmp");std::fs::write(&temp,serde_json::to_vec_pretty(self)?)?;std::fs::rename(temp,path)
    }
}
