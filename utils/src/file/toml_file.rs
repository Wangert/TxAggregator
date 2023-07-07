use std::{fs::File, io::Read};

use serde::de::DeserializeOwned;

use super::error::FileError;

pub fn toml_file_read<T>(file_path: &str) -> Result<T, FileError> 
where
    T: DeserializeOwned,
{
    let mut file = File::open(file_path).map_err(|e| FileError::file_read(file_path.to_string(), e))?;
    let mut str_val = String::new();

    file.read_to_string(&mut str_val).map_err(|e| FileError::file_read_to_string(e))?;

    let result: T = toml::from_str(&str_val).map_err(|e| FileError::parse_toml_file_from_string(file_path.to_string(), e))?;

    Ok(result)
}