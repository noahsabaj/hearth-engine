//! Renderer Error - Stub
pub type RenderResult<T> = Result<T, String>;
pub type RendererResult<T> = Result<T, String>;

pub struct RendererErrorContext;

pub fn buffer_mapping_error(_msg: &str) -> String {
    String::from("Buffer mapping error")
}
