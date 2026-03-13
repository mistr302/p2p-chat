use std::{collections::HashMap, path::PathBuf};

use base64::{Engine as _, engine::general_purpose};
use directories::ProjectDirs;
use libp2p::identity::Keypair;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::fs::read_to_string;

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
    Bytes(Option<Vec<u8>>),
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
    KeyPair,
}

pub trait Generateable {
    fn generate_value() -> SettingValue;
}

#[derive(Clone, Copy)]
pub enum SettingInput {
    HumanInput,
    Generated(fn() -> SettingValue),
}

pub struct SettingDefinition {
    pub name: SettingName,
    pub label: &'static str,
    pub default_value: SettingValue,
    pub input: SettingInput,
}

pub struct KeyPairSetting;

impl Generateable for KeyPairSetting {
    fn generate_value() -> SettingValue {
        let key = Keypair::generate_ed25519();
        let bytes = key.to_protobuf_encoding().expect("keypair encoding failed");
        let encoded = general_purpose::STANDARD.encode(bytes);
        SettingValue::String(Some(encoded))
    }
}

static SETTING_DEFINITIONS: &[SettingDefinition] = &[
    SettingDefinition {
        name: SettingName::Name,
        label: "Name",
        default_value: SettingValue::String(None),
        input: SettingInput::HumanInput,
    },
    SettingDefinition {
        name: SettingName::KeyPair,
        label: "Keypair",
        default_value: SettingValue::String(None),
        input: SettingInput::Generated(KeyPairSetting::generate_value),
    },
];

pub fn setting_definitions() -> &'static [SettingDefinition] {
    SETTING_DEFINITIONS
}
pub struct Settings;
impl Settings {
    pub fn load() -> HashMap<SettingName, SettingValue> {
        let settings: HashMap<SettingName, SettingValue> = SETTING_DEFINITIONS
            .iter()
            .map(|def| (def.name, def.default_value.clone()))
            .collect();
        // TODO: If there is no configuration we can return
        let settings_path = get_save_file_path(SaveFile::Settings);
        let settings_json = read_to_string(&settings_path);
        let json = match settings_json {
            Ok(settings) => settings,
            Err(err) => {
                std::fs::File::create(settings_path.clone()).unwrap();
                tracing::error!("{:?}", err);
                tracing::warn!("Defaulting to predefined settings");
                return settings;
            }
        };
        let mut user_settings =
            match serde_json::from_str::<HashMap<SettingName, SettingValue>>(&json) {
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
        if let Some(SettingValue::Bytes(Some(bytes))) =
            user_settings.get(&SettingName::KeyPair).cloned()
        {
            let encoded = general_purpose::STANDARD.encode(bytes);
            user_settings.insert(SettingName::KeyPair, SettingValue::String(Some(encoded)));
        }
        user_settings
    }
    pub fn save(settings: &HashMap<SettingName, SettingValue>) {
        let settings_path = get_save_file_path(SaveFile::Settings);
        tracing::info!("saving to path: {:?}", settings_path);
        let serialized = serde_json::to_string::<HashMap<SettingName, SettingValue>>(settings)
            .expect("failed to serialize settings");
        std::fs::write(settings_path, serialized).expect("failed to write settings");
    }
}
pub(crate) fn create_project_dirs() -> std::io::Result<()> {
    let proj_dir =
        ProjectDirs::from("com", "Mistr", "p2pchat").expect("Couldnt determine directories");
    create_dir_all(proj_dir.config_dir())?;
    create_dir_all(proj_dir.data_dir())?;
    Ok(())
}

pub(crate) fn get_save_file_path(savefile: SaveFile) -> PathBuf {
    let proj_dirs =
        ProjectDirs::from("com", "Mistr", "p2pchat").expect("Couldnt determine directories");
    let file_name = SAVE_FILES
        .iter()
        .find(|x| x.0 == savefile)
        .expect("Save file path not defined")
        .1;
    match savefile {
        SaveFile::Settings => proj_dirs.config_dir().join(file_name),
        SaveFile::Database => proj_dirs.data_dir().join(file_name),
    }
}
#[derive(PartialEq)]
pub(crate) enum SaveFile {
    Settings,
    Database,
}
static SAVE_FILES: &[(SaveFile, &str)] =
    &[(SaveFile::Settings, "settings"), (SaveFile::Database, "db")];
