//! Texture Atlas Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::texture_atlas_data::{AtlasUV, MaterialId, MaterialLibrary, PackedRect, TextureAtlasData};
use cgmath::Vector2;
use image::{DynamicImage, RgbaImage};
use wgpu::{Device, Queue};

/// Transform local UV (0-1) to atlas UV
pub fn transform_uv(atlas_uv: &AtlasUV, local_uv: Vector2<f32>) -> Vector2<f32> {
    Vector2::new(
        atlas_uv.min.x + (atlas_uv.max.x - atlas_uv.min.x) * local_uv.x,
        atlas_uv.min.y + (atlas_uv.max.y - atlas_uv.min.y) * local_uv.y,
    )
}

/// Create new texture atlas data
pub fn create_texture_atlas(device: &Device, atlas_size: u32, tile_size: u32) -> TextureAtlasData {
    let padding = 2; // 2 pixel padding to prevent bleeding

    // Get device limits to ensure we don't exceed GPU capabilities
    let device_limits = device.limits();
    let max_dimension = device_limits.max_texture_dimension_2d;

    // Validate and clamp atlas size
    let clamped_atlas_size = atlas_size.min(max_dimension);

    // Log if dimensions were clamped
    if clamped_atlas_size != atlas_size {
        log::warn!(
            "[texture_atlas_operations::create] Atlas size clamped from {} to {} due to GPU limits (max: {})",
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

    TextureAtlasData {
        texture,
        view,
        sampler,
        atlas_size: clamped_atlas_size,
        tile_size,
        padding,
        material_uvs: std::collections::HashMap::new(),
        next_material_id: 1,
        packed_rects: Vec::new(),
        atlas_image,
        dirty: false,
    }
}

/// Add texture to atlas
pub fn add_texture(data: &mut TextureAtlasData, image: &DynamicImage) -> MaterialId {
    let rgba = image.to_rgba8();
    let width = rgba.width().min(data.tile_size);
    let height = rgba.height().min(data.tile_size);

    // Find packing position
    let rect = find_packing_position(data, width + data.padding * 2, height + data.padding * 2);

    if let Some(rect) = rect {
        // Copy image to atlas (with padding offset)
        for y in 0..height {
            for x in 0..width {
                let pixel = rgba.get_pixel(x, y);
                data.atlas_image.put_pixel(
                    rect.x + data.padding + x,
                    rect.y + data.padding + y,
                    *pixel,
                );
            }
        }

        // Calculate UV coordinates (accounting for padding)
        let uv = AtlasUV {
            min: Vector2::new(
                (rect.x + data.padding) as f32 / data.atlas_size as f32,
                (rect.y + data.padding) as f32 / data.atlas_size as f32,
            ),
            max: Vector2::new(
                (rect.x + data.padding + width) as f32 / data.atlas_size as f32,
                (rect.y + data.padding + height) as f32 / data.atlas_size as f32,
            ),
        };

        let material_id = data.next_material_id;
        data.next_material_id += 1;

        data.material_uvs.insert(material_id, uv);
        data.packed_rects.push(rect);
        data.dirty = true;

        material_id
    } else {
        // Atlas is full
        0 // Return default material
    }
}

/// Add multiple textures from a tileset
pub fn add_tileset(
    data: &mut TextureAtlasData,
    tileset: &DynamicImage,
    tiles_x: u32,
    tiles_y: u32,
) -> Vec<MaterialId> {
    let mut material_ids = Vec::new();

    let tile_width = tileset.width() / tiles_x;
    let tile_height = tileset.height() / tiles_y;

    for y in 0..tiles_y {
        for x in 0..tiles_x {
            let tile = tileset.crop_imm(x * tile_width, y * tile_height, tile_width, tile_height);

            let id = add_texture(data, &tile);
            material_ids.push(id);
        }
    }

    material_ids
}

/// Find position to pack new rectangle
fn find_packing_position(data: &TextureAtlasData, width: u32, height: u32) -> Option<PackedRect> {
    // Simple row packing algorithm
    let mut y = 0;
    let mut row_height = 0;

    for rect in &data.packed_rects {
        if rect.y != y {
            // New row
            y = rect.y;
            row_height = rect.height;
        }

        row_height = row_height.max(rect.height);
    }

    // Try to add to current row
    let mut x = 0;
    for rect in &data.packed_rects {
        if rect.y == y {
            x = x.max(rect.x + rect.width);
        }
    }

    if x + width <= data.atlas_size {
        // Fits in current row
        return Some(PackedRect {
            x,
            y,
            width,
            height,
        });
    }

    // Start new row
    y += row_height;
    if y + height <= data.atlas_size {
        return Some(PackedRect {
            x: 0,
            y,
            width,
            height,
        });
    }

    // Atlas is full
    None
}

/// Upload atlas to GPU if dirty
pub fn upload_atlas(data: &mut TextureAtlasData, queue: &Queue) {
    if !data.dirty {
        return;
    }

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &data.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data.atlas_image,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * data.atlas_size),
            rows_per_image: Some(data.atlas_size),
        },
        wgpu::Extent3d {
            width: data.atlas_size,
            height: data.atlas_size,
            depth_or_array_layers: 1,
        },
    );

    data.dirty = false;
}

/// Get UV coordinates for material
pub fn get_uv(data: &TextureAtlasData, material_id: MaterialId) -> Option<AtlasUV> {
    data.material_uvs.get(&material_id).copied()
}

/// Get texture view for binding
pub fn texture_view(data: &TextureAtlasData) -> &wgpu::TextureView {
    &data.view
}

/// Get sampler for binding
pub fn sampler(data: &TextureAtlasData) -> &wgpu::Sampler {
    &data.sampler
}

/// Get atlas utilization percentage
pub fn utilization(data: &TextureAtlasData) -> f32 {
    let used_area: u32 = data.packed_rects.iter().map(|r| r.width * r.height).sum();

    let total_area = data.atlas_size * data.atlas_size;
    (used_area as f32 / total_area as f32) * 100.0
}

/// Save atlas to file for debugging
pub fn save_debug(data: &TextureAtlasData, path: &str) -> Result<(), image::ImageError> {
    data.atlas_image.save(path)
}

/// Create solid color texture for testing
pub fn create_solid_color(r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = RgbaImage::new(16, 16);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([r, g, b, 255]);
    }
    DynamicImage::ImageRgba8(img)
}

/// Load default materials into atlas
pub fn load_default_materials(data: &mut TextureAtlasData) -> MaterialLibrary {
    // In a real implementation, load actual textures
    // For now, create placeholder colors

    let stone = add_texture(data, &create_solid_color(128, 128, 128));
    let dirt = add_texture(data, &create_solid_color(101, 67, 33));
    let grass_top = add_texture(data, &create_solid_color(0, 154, 23));
    let grass_side = add_texture(data, &create_solid_color(0, 154, 23));
    let sand = add_texture(data, &create_solid_color(194, 178, 128));
    let water = add_texture(data, &create_solid_color(64, 164, 223));
    let wood = add_texture(data, &create_solid_color(139, 69, 19));
    let leaves = add_texture(data, &create_solid_color(34, 139, 34));

    MaterialLibrary {
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
