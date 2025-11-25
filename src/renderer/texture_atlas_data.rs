//! Texture Atlas Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in texture_atlas_operations.rs

use cgmath::Vector2;
use image::RgbaImage;
use std::collections::HashMap;
use wgpu::{Sampler, Texture, TextureView};

/// UV coordinates within the atlas
#[derive(Debug, Clone, Copy)]
pub struct AtlasUV {
    pub min: Vector2<f32>,
    pub max: Vector2<f32>,
}

/// Rectangle packing for atlas
#[derive(Debug, Clone)]
pub struct PackedRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Material ID to atlas UV mapping
pub type MaterialId = u32;

/// Texture atlas data - Pure data structure
pub struct TextureAtlasData {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,

    pub atlas_size: u32,
    pub tile_size: u32,
    pub padding: u32,

    pub material_uvs: HashMap<MaterialId, AtlasUV>,
    pub next_material_id: MaterialId,

    // Packing state
    pub packed_rects: Vec<PackedRect>,
    pub atlas_image: RgbaImage,
    pub dirty: bool,
}

/// Pre-defined material mappings
pub struct MaterialLibrary {
    pub stone: MaterialId,
    pub dirt: MaterialId,
    pub grass_top: MaterialId,
    pub grass_side: MaterialId,
    pub sand: MaterialId,
    pub water: MaterialId,
    pub wood: MaterialId,
    pub leaves: MaterialId,
}
