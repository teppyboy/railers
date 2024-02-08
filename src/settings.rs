#[derive(Clone, Default, Debug)]
pub struct MapStoryKey {
    pub f_enabled: bool,
    pub enter_enabled: bool,
    pub custom_key_enabled: bool,
    pub custom_key: Option<String>,
}
#[derive(Clone, Default, Debug)]
pub struct Utils {
    pub msk: MapStoryKey,
}

#[derive(Clone, Default, Debug)]
pub struct Settings {
    pub utils: Utils,
}
