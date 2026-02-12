use std::{collections::HashMap, path::PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use tokio::fs::read_to_string;

#[derive(Serialize, Deserialize, Clone, Debug)]
enum Constraint {
    MaxValue(i32),
    MinValue(i32),
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SettingValue {
    Int(i32),
    Bool(bool),
    String(Option<String>),
}
impl TryInto<String> for SettingValue {
    type Error = std::io::Error;
    fn try_into(self) -> Result<String, Self::Error> {
        if let SettingValue::String(Some(str)) = self {
            Ok(str)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Could not parse into String",
            ))
        }
    }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SettingName {
    Name,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Setting {
    constraints: Option<Vec<Constraint>>,
    value: SettingValue,
}
impl Setting {
    pub fn get_value(&self) -> &SettingValue {
        &self.value
    }
    // TODO: Implement error if value doesnt comply with constraints
    // std::io::ErrorKind::InvalidInput
    pub fn set_value(&mut self, val: SettingValue) -> std::io::Result<()> {
        self.value = val;
        Ok(())
    }
}
pub struct Settings;
impl Settings {
    pub async fn load() -> HashMap<SettingName, Setting> {
        let settings: HashMap<SettingName, Setting> = REQUIRED_SETTINGS
            .iter()
            .map(|(name, setting)| (*name, setting.clone()))
            .collect();
        create_config_path().unwrap();
        // TODO: If there is no configuration we can return
        let settings_path = get_config_save_file_path(SaveFile::Settings);
        let settings_json = read_to_string(&settings_path).await;
        let json = match settings_json {
            Ok(settings) => settings,
            Err(err) => {
                tokio::fs::File::create(settings_path.clone())
                    .await
                    .unwrap();
                tracing::error!("{:?}", err);
                tracing::warn!("Defaulting to predefined settings");
                return settings;
            }
        };
        let mut user_settings = match serde_json::from_str::<HashMap<SettingName, Setting>>(&json) {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("{:?}", err);
                tracing::warn!("Defaulting to predefined settings");
                return settings;
            }
        };

        // TODO: Set default values to missing options, enforce constraints
        for (opt_key, opt_val) in settings {
            if let Some(setting) = user_settings.get_mut(&opt_key) {
                // TODO: enforce the constraints here
            } else {
                // insert the default if opt is missing
                user_settings.insert(opt_key, opt_val);
            }
        }
        user_settings
    }
    pub async fn save(settings: &HashMap<SettingName, Setting>) {
        let settings_path = get_config_save_file_path(SaveFile::Settings);
        tracing::info!("saving to path: {:?}", settings_path);
        let serialized = serde_json::to_string::<HashMap<SettingName, Setting>>(settings)
            .expect("failed to serialize settings");
        std::fs::write(settings_path, serialized).expect("failed to write settings");
    }
}
fn create_config_path() -> std::io::Result<()> {
    let proj_dir =
        ProjectDirs::from("com", "Mistr", "p2pchat").expect("Couldnt determine directories");
    create_dir_all(proj_dir.config_dir())?;
    Ok(())
}

pub(crate) fn get_config_save_file_path(savefile: SaveFile) -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "Mistr", "p2pchat").expect("Couldnt determine directories");
    proj_dirs.config_dir().join(
        SAVE_FILES
            .iter()
            .find(|x| x.0 == savefile)
            .expect("Save file path not defined")
            .1,
    )
}
static REQUIRED_SETTINGS: &[(SettingName, Setting)] = &[(
    SettingName::Name,
    Setting {
        constraints: None,
        value: SettingValue::String(None),
    },
)];
#[derive(PartialEq)]
pub(crate) enum SaveFile {
    Settings,
    Database,
}
static SAVE_FILES: &[(SaveFile, &str)] =
    &[(SaveFile::Settings, "settings"), (SaveFile::Database, "db")];
