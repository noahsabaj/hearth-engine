//! Transform Stage Data - Stub
pub struct TransformStageData;

#[derive(Debug)]
pub struct ActualOutput {
    pub output_type: OutputType,
    pub quantity: u32,
    pub quality: f32,
}

#[derive(Debug)]
pub enum OutputType {
    Primary,
    Secondary,
    Item(u32), // Resource ID
}

pub struct StageOutput;
pub struct StageRequirement;
pub struct TransformStage {
    pub duration: f32,
}
pub struct ValidationContext;
pub enum ValidationResult { Valid, Invalid }
pub struct ItemRequirement;
pub struct ToolRequirement;
pub struct EnvironmentRequirement;
pub enum WeatherType { Clear, Rain, Snow }
