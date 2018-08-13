use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    pub src: Src,
    pub build: Build,
    pub link: Link,
    #[serde(default)]
    pub info: Info,
    #[serde(default)]
    pub files: HashMap<String, PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Src {
    pub src: Option<PathBuf>,
    pub iso: PathBuf,
    pub patch: Option<PathBuf>,
    pub map: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Info {
    pub game_name: Option<String>,
    pub developer_name: Option<String>,
    pub full_game_name: Option<String>,
    pub full_developer_name: Option<String>,
    pub description: Option<String>,
    pub image: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct Build {
    pub map: PathBuf,
    pub iso: PathBuf,
}

#[derive(Deserialize)]
pub struct Link {
    pub entries: Vec<String>,
    pub base: String,
    pub libs: Option<Vec<PathBuf>>,
}
