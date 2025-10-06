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
