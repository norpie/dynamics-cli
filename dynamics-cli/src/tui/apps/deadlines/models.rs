/// Parameters passed from EnvironmentSelectApp to FileSelectApp
#[derive(Clone, Debug)]
pub struct FileSelectParams {
    pub environment_name: String,
}

impl Default for FileSelectParams {
    fn default() -> Self {
        Self {
            environment_name: String::new(),
        }
    }
}

/// Parameters passed from FileSelectApp to MappingApp
#[derive(Clone, Debug)]
pub struct MappingParams {
    pub environment_name: String,
    pub file_path: std::path::PathBuf,
    pub sheet_name: String,
}

impl Default for MappingParams {
    fn default() -> Self {
        Self {
            environment_name: String::new(),
            file_path: std::path::PathBuf::new(),
            sheet_name: String::new(),
        }
    }
}
