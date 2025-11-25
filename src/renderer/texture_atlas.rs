/// Texture Atlas System
/// 
/// Manages material textures in a single atlas for efficient rendering.
/// Supports dynamic packing and UV coordinate generation.

use wgpu::{Device, Queue, Texture, TextureView, Sampler};
use image::{DynamicImage, RgbaImage};
use std::collections::HashMap;
use super::preallocated_texture_atlas::{PreallocatedTextureAtlas, MAX_MATERIALS};
use cgmath::Vector2;

/// UV coordinates within the atlas
#[derive(Debug, Clone, Copy)]
pub struct AtlasUV {
    pub min: Vector2<f32>,
    pub max: Vector2<f32>,
}

impl AtlasUV {
    /// Transform local UV (0-1) to atlas UV
    pub fn transform(&self, local_uv: Vector2<f32>) -> Vector2<f32> {
        Vector2::new(
            self.min.x + (self.max.x - self.min.x) * local_uv.x,
            self.min.y + (self.max.y - self.min.y) * local_uv.y,
        )
    }
}

/// Rectangle packing for atlas
#[derive(Debug, Clone)]
struct PackedRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Material ID to atlas UV mapping
pub type MaterialId = u32;

/// Texture atlas for efficient GPU rendering
pub struct TextureAtlas {
    texture: Texture,
    view: TextureView,
    sampler: Sampler,
    
    atlas_size: u32,
    tile_size: u32,
    padding: u32,
    
    material_uvs: HashMap<MaterialId, AtlasUV>,
    next_material_id: MaterialId,
    
    // Packing state
    packed_rects: Vec<PackedRect>,
    atlas_image: RgbaImage,
    dirty: bool,
}

impl TextureAtlas {
    /// Create new texture atlas
    pub fn new(device: &Device, atlas_size: u32, tile_size: u32) -> Self {
        let padding = 2; // 2 pixel padding to prevent bleeding
        
        // Get device limits to ensure we don't exceed GPU capabilities
        let device_limits = device.limits();
        let max_dimension = device_limits.max_texture_dimension_2d;
        
        // Validate and clamp atlas size
        let clamped_atlas_size = atlas_size.min(max_dimension);
        
        // Log if dimensions were clamped
        if clamped_atlas_size != atlas_size {
            log::warn!(
                "[TextureAtlas::new] Atlas size clamped from {} to {} due to GPU limits (max: {})",
                atlas_size, clamped_atlas_size, max_dimension
            );
        }
        
        // Create atlas texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture Atlas"),
            size: wgpu::Extent3d {
                width: clamped_atlas_size,
                height: clamped_atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create sampler with filtering
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        let atlas_image = RgbaImage::new(clamped_atlas_size, clamped_atlas_size);
        
        Self {
            texture,
            view,
            sampler,
            atlas_size: clamped_atlas_size,
            tile_size,
            padding,
            material_uvs: HashMap::new(),
            next_material_id: 1,
            packed_rects: Vec::new(),
            atlas_image,
            dirty: false,
        }
    }
    
    /// Add texture to atlas
    pub fn add_texture(&mut self, image: &DynamicImage) -> MaterialId {
        let rgba = image.to_rgba8();
        let width = rgba.width().min(self.tile_size);
        let height = rgba.height().min(self.tile_size);
        
        // Find packing position
        let rect = self.find_packing_position(width + self.padding * 2, height + self.padding * 2);
        
        if let Some(rect) = rect {
            // Copy image to atlas (with padding offset)
            for y in 0..height {
                for x in 0..width {
                    let pixel = rgba.get_pixel(x, y);
                    self.atlas_image.put_pixel(
                        rect.x + self.padding + x,
                        rect.y + self.padding + y,
                        *pixel,
                    );
                }
            }
            
            // Calculate UV coordinates (accounting for padding)
            let uv = AtlasUV {
                min: Vector2::new(
                    (rect.x + self.padding) as f32 / self.atlas_size as f32,
                    (rect.y + self.padding) as f32 / self.atlas_size as f32,
                ),
                max: Vector2::new(
                    (rect.x + self.padding + width) as f32 / self.atlas_size as f32,
                    (rect.y + self.padding + height) as f32 / self.atlas_size as f32,
                ),
            };
            
            let material_id = self.next_material_id;
            self.next_material_id += 1;
            
            self.material_uvs.insert(material_id, uv);
            self.packed_rects.push(rect);
            self.dirty = true;
            
            material_id
        } else {
            // Atlas is full
            0 // Return default material
        }
    }
    
    /// Add multiple textures from a tileset
    pub fn add_tileset(&mut self, tileset: &DynamicImage, tiles_x: u32, tiles_y: u32) -> Vec<MaterialId> {
        let mut material_ids = Vec::new();
        
        let tile_width = tileset.width() / tiles_x;
        let tile_height = tileset.height() / tiles_y;
        
        for y in 0..tiles_y {
            for x in 0..tiles_x {
                let tile = tileset.crop_imm(
                    x * tile_width,
                    y * tile_height,
                    tile_width,
                    tile_height,
                );
                
                let id = self.add_texture(&tile);
                material_ids.push(id);
            }
        }
        
        material_ids
    }
    
    /// Find position to pack new rectangle
    fn find_packing_position(&self, width: u32, height: u32) -> Option<PackedRect> {
        // Simple row packing algorithm
        let mut y = 0;
        let mut row_height = 0;
        
        for rect in &self.packed_rects {
            if rect.y != y {
                // New row
                y = rect.y;
                row_height = rect.height;
            }
            
            row_height = row_height.max(rect.height);
        }
        
        // Try to add to current row
        let mut x = 0;
        for rect in &self.packed_rects {
            if rect.y == y {
                x = x.max(rect.x + rect.width);
            }
        }
        
        if x + width <= self.atlas_size {
            // Fits in current row
            return Some(PackedRect { x, y, width, height });
        }
        
        // Start new row
        y += row_height;
        if y + height <= self.atlas_size {
            return Some(PackedRect { x: 0, y, width, height });
        }
        
        // Atlas is full
        None
    }
    
    /// Upload atlas to GPU if dirty
    pub fn upload(&mut self, queue: &Queue) {
        if !self.dirty {
            return;
        }
        
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.atlas_image,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.atlas_size),
                rows_per_image: Some(self.atlas_size),
            },
            wgpu::Extent3d {
                width: self.atlas_size,
                height: self.atlas_size,
                depth_or_array_layers: 1,
            },
        );
        
        self.dirty = false;
    }
    
    /// Get UV coordinates for material
    pub fn get_uv(&self, material_id: MaterialId) -> Option<AtlasUV> {
        self.material_uvs.get(&material_id).copied()
    }
    
    /// Get texture view for binding
    pub fn texture_view(&self) -> &TextureView {
        &self.view
    }
    
    /// Get sampler for binding
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }
    
    /// Get atlas utilization percentage
    pub fn utilization(&self) -> f32 {
        let used_area: u32 = self.packed_rects.iter()
            .map(|r| r.width * r.height)
            .sum();
        
        let total_area = self.atlas_size * self.atlas_size;
        (used_area as f32 / total_area as f32) * 100.0
    }
    
    /// Save atlas to file for debugging
    pub fn save_debug(&self, path: &str) -> Result<(), image::ImageError> {
        self.atlas_image.save(path)
    }
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

impl MaterialLibrary {
    /// Load default materials into atlas
    pub fn load_defaults(atlas: &mut TextureAtlas) -> Self {
        // In a real implementation, load actual textures
        // For now, create placeholder colors
        
        let stone = atlas.add_texture(&create_solid_color(128, 128, 128));
        let dirt = atlas.add_texture(&create_solid_color(101, 67, 33));
        let grass_top = atlas.add_texture(&create_solid_color(0, 154, 23));
        let grass_side = atlas.add_texture(&create_solid_color(0, 154, 23));
        let sand = atlas.add_texture(&create_solid_color(194, 178, 128));
        let water = atlas.add_texture(&create_solid_color(64, 164, 223));
        let wood = atlas.add_texture(&create_solid_color(139, 69, 19));
        let leaves = atlas.add_texture(&create_solid_color(34, 139, 34));
        
        Self {
            stone,
            dirt,
            grass_top,
            grass_side,
            sand,
            water,
            wood,
            leaves,
        }
    }
}

/// Create solid color texture for testing
fn create_solid_color(r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = RgbaImage::new(16, 16);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([r, g, b, 255]);
    }
    DynamicImage::ImageRgba8(img)
}