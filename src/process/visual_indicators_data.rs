//! Visual Indicators Data - Stub
pub struct VisualIndicatorsData;

pub struct ProcessVisual;

impl ProcessVisual {
    pub fn default() -> Self {
        Self
    }
}
pub struct ProgressBar;
pub struct StatusIcon;
pub enum ProgressColor { Green, Yellow, Red }
pub enum BarAnimation { None, Pulse }
pub struct TextOverlay;
pub enum TextPosition { Top, Bottom }
pub struct TextStyle;
pub struct ParticleEffect;
pub enum ParticleType { Smoke, Sparkle }
pub enum AnimationState { Idle, Active, Complete }
