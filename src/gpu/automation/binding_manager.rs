//! Centralized GPU binding management system
//!
//! This module provides automatic binding index management, eliminating manual
//! binding constants and ensuring no conflicts across shaders.

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Mutex;
use wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, ShaderStages,
};

// Global binding registry for automatic binding management
lazy_static! {
    static ref BINDING_REGISTRY: Mutex<BindingRegistry> = Mutex::new(BindingRegistry::new());
}

/// Type-safe binding key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BindingKey {
    pub group: u32,
    pub name: String,
    pub stage: ShaderStages,
}

/// Binding information
#[derive(Debug, Clone)]
pub struct BindingInfo {
    pub binding: u32,
    pub ty: BindingType,
    pub visibility: ShaderStages,
}

/// Registry for managing all GPU bindings
pub struct BindingRegistry {
    /// Map from binding key to binding info
    bindings: HashMap<BindingKey, BindingInfo>,
    /// Next available binding index per group
    next_binding: HashMap<u32, u32>,
}

impl BindingRegistry {
    fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            next_binding: HashMap::new(),
        }
    }

    /// Register a binding and get its index
    pub fn register_binding(
        &mut self,
        group: u32,
        name: impl Into<String>,
        ty: BindingType,
        visibility: ShaderStages,
    ) -> u32 {
        let key = BindingKey {
            group,
            name: name.into(),
            stage: visibility,
        };

        // Check if already registered
        if let Some(info) = self.bindings.get(&key) {
            return info.binding;
        }

        // Get next available binding index for this group
        let binding = self.next_binding.get(&group).copied().unwrap_or(0);
        self.next_binding.insert(group, binding + 1);

        // Register the binding
        self.bindings.insert(
            key,
            BindingInfo {
                binding,
                ty,
                visibility,
            },
        );

        binding
    }

    /// Get all bindings for a group
    pub fn get_group_bindings(&self, group: u32) -> Vec<BindGroupLayoutEntry> {
        let mut entries: Vec<_> = self
            .bindings
            .iter()
            .filter(|(key, _)| key.group == group)
            .map(|(_, info)| BindGroupLayoutEntry {
                binding: info.binding,
                visibility: info.visibility,
                ty: info.ty,
                count: None,
            })
            .collect();

        // Sort by binding index for consistency
        entries.sort_by_key(|e| e.binding);
        entries
    }

    /// Generate WGSL binding declarations for a group
    pub fn generate_wgsl_bindings(&self, group: u32) -> String {
        let mut wgsl = String::new();

        let mut bindings: Vec<_> = self
            .bindings
            .iter()
            .filter(|(key, _)| key.group == group)
            .collect();

        // Sort by binding index
        bindings.sort_by_key(|(_, info)| info.binding);

        for (key, info) in bindings {
            let binding_str = format!(
                "@group({}) @binding({}) var{} {}: {};\n",
                group,
                info.binding,
                binding_type_to_wgsl_qualifier(&info.ty),
                key.name.to_lowercase().replace(' ', "_"),
                binding_type_to_wgsl_type(&key.name, &info.ty),
            );
            wgsl.push_str(&binding_str);
        }

        wgsl
    }
}

/// Convert binding type to WGSL storage qualifier
fn binding_type_to_wgsl_qualifier(ty: &BindingType) -> &'static str {
    match ty {
        BindingType::Buffer { ty, .. } => match ty {
            wgpu::BufferBindingType::Uniform => "",
            wgpu::BufferBindingType::Storage { read_only: true } => "<storage, read>",
            wgpu::BufferBindingType::Storage { read_only: false } => "<storage, read_write>",
        },
        BindingType::Texture { .. } => "",
        BindingType::Sampler { .. } => "",
        _ => "",
    }
}

/// Convert binding type to WGSL type name
fn binding_type_to_wgsl_type(name: &str, ty: &BindingType) -> String {
    match ty {
        BindingType::Buffer { .. } => {
            // Try to infer type from name
            if name.contains("camera") {
                "CameraUniform".to_string()
            } else if name.contains("instance") {
                "array<InstanceData>".to_string()
            } else if name.contains("world") {
                "array<u32>".to_string()
            } else {
                // Default to generic buffer
                "array<vec4<f32>>".to_string()
            }
        }
        BindingType::Texture { .. } => "texture_2d<f32>".to_string(),
        BindingType::Sampler { .. } => "sampler".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Macro to define bindings with automatic index assignment
#[macro_export]
macro_rules! define_bindings {
    (
        group = $group:expr,
        bindings = {
            $( $name:ident : $ty:expr => $visibility:expr ),* $(,)?
        }
    ) => {
        pub mod bindings {
            use super::*;
            use $crate::gpu::binding_manager::{register_binding, BindingType, ShaderStages};

            lazy_static::lazy_static! {
                $(
                    pub static ref $name: u32 = register_binding(
                        $group,
                        stringify!($name),
                        $ty,
                        $visibility,
                    );
                )*
            }

            /// Get bind group layout entries for this group
            pub fn get_layout_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
                $crate::gpu::binding_manager::get_group_layout_entries($group)
            }

            /// Generate WGSL binding declarations
            pub fn generate_wgsl() -> String {
                $crate::gpu::binding_manager::generate_group_wgsl($group)
            }
        }
    };
}

/// Register a binding in the global registry
pub fn register_binding(
    group: u32,
    name: impl Into<String>,
    ty: BindingType,
    visibility: ShaderStages,
) -> u32 {
    BINDING_REGISTRY
        .lock()
        .expect("[BindingManager] Failed to acquire binding registry lock")
        .register_binding(group, name, ty, visibility)
}

/// Get bind group layout entries for a group
pub fn get_group_layout_entries(group: u32) -> Vec<BindGroupLayoutEntry> {
    BINDING_REGISTRY
        .lock()
        .expect("[BindingManager] Failed to acquire binding registry lock")
        .get_group_bindings(group)
}

/// Generate WGSL bindings for a group
pub fn generate_group_wgsl(group: u32) -> String {
    BINDING_REGISTRY
        .lock()
        .expect("[BindingManager] Failed to acquire binding registry lock")
        .generate_wgsl_bindings(group)
}

/// Create a bind group layout from registered bindings
pub fn create_bind_group_layout(
    device: &wgpu::Device,
    group: u32,
    label: Option<&str>,
) -> BindGroupLayout {
    let entries = get_group_layout_entries(group);

    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label,
        entries: &entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::{BufferBindingType, ShaderStages};

    #[test]
    fn test_binding_registration() {
        let mut registry = BindingRegistry::new();

        // Register some bindings
        let binding1 = registry.register_binding(
            0,
            "camera",
            BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
        );

        let binding2 = registry.register_binding(
            0,
            "instances",
            BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            ShaderStages::VERTEX,
        );

        // Bindings should be sequential
        assert_eq!(binding1, 0);
        assert_eq!(binding2, 1);

        // Re-registering should return same index
        let binding1_again = registry.register_binding(
            0,
            "camera",
            BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            ShaderStages::VERTEX | ShaderStages::FRAGMENT,
        );
        assert_eq!(binding1_again, binding1);
    }

    #[test]
    fn test_wgsl_generation() {
        let mut registry = BindingRegistry::new();

        registry.register_binding(
            0,
            "world_data",
            BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            ShaderStages::COMPUTE,
        );

        let wgsl = registry.generate_wgsl_bindings(0);
        assert!(wgsl.contains("@group(0) @binding(0)"));
        assert!(wgsl.contains("var<storage, read_write>"));
        assert!(wgsl.contains("world_data"));
    }
}
