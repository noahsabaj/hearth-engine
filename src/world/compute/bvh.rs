use crate::memory::MemoryManager;
use crate::world::core::ChunkPos;
use bytemuck::{Pod, Zeroable};
use cgmath::{Point3, Vector3};
/// Bounding Volume Hierarchy for Ray Tracing Support

use std::sync::Arc;
use wgpu::{Buffer, Device, Queue};

/// BVH node format optimized for GPU traversal
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BvhNode {
    /// Bounding box min
    pub aabb_min: [f32; 3],
    /// Left child index or primitive index if leaf
    pub left_first: u32,
    /// Bounding box max
    pub aabb_max: [f32; 3],
    /// Primitive count (0 for internal nodes)
    pub prim_count: u32,
}

impl BvhNode {
    pub fn is_leaf(&self) -> bool {
        self.prim_count > 0
    }

    pub fn intersect_ray(
        &self,
        ray_origin: &Vector3<f32>,
        ray_inv_dir: &Vector3<f32>,
    ) -> Option<f32> {
        let t1 = Vector3::new(
            (self.aabb_min[0] - ray_origin.x) * ray_inv_dir.x,
            (self.aabb_min[1] - ray_origin.y) * ray_inv_dir.y,
            (self.aabb_min[2] - ray_origin.z) * ray_inv_dir.z,
        );
        let t2 = Vector3::new(
            (self.aabb_max[0] - ray_origin.x) * ray_inv_dir.x,
            (self.aabb_max[1] - ray_origin.y) * ray_inv_dir.y,
            (self.aabb_max[2] - ray_origin.z) * ray_inv_dir.z,
        );

        let tmin = Vector3::new(t1.x.min(t2.x), t1.y.min(t2.y), t1.z.min(t2.z));
        let tmax = Vector3::new(t1.x.max(t2.x), t1.y.max(t2.y), t1.z.max(t2.z));

        let tmin_scalar = tmin.x.max(tmin.y).max(tmin.z);
        let tmax_scalar = tmax.x.min(tmax.y).min(tmax.z);

        if tmax_scalar >= tmin_scalar && tmax_scalar >= 0.0 {
            Some(tmin_scalar.max(0.0))
        } else {
            None
        }
    }
}

/// Primitive reference for BVH construction
#[derive(Clone, Debug)]
struct Primitive {
    center: Point3<f32>,
    aabb_min: Point3<f32>,
    aabb_max: Point3<f32>,
    index: u32,
}

/// BVH for voxel chunks and instances
pub struct VoxelBvh {
    device: Arc<Device>,

    /// GPU buffer containing BVH nodes
    node_buffer: Buffer,

    /// GPU buffer containing primitive indices
    primitive_buffer: Buffer,

    /// BVH statistics
    node_count: u32,
    primitive_count: u32,
    max_depth: u32,
}

impl VoxelBvh {
    pub fn new(
        device: Arc<Device>,
        memory_manager: &mut MemoryManager,
        max_primitives: u32,
    ) -> Self {
        // Allocate buffers for worst-case BVH size
        let max_nodes = max_primitives * 2;
        let node_buffer_size = max_nodes as u64 * std::mem::size_of::<BvhNode>() as u64;
        let primitive_buffer_size = max_primitives as u64 * std::mem::size_of::<u32>() as u64;

        let node_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BVH Node Buffer"),
            size: node_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let primitive_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BVH Primitive Buffer"),
            size: primitive_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            node_buffer,
            primitive_buffer,
            node_count: 0,
            primitive_count: 0,
            max_depth: 0,
        }
    }

    /// Build BVH from chunk positions
    pub fn build_from_chunks(
        &mut self,
        queue: &Queue,
        chunk_positions: &[ChunkPos],
        chunk_size: f32,
    ) {
        // Convert chunks to primitives
        let mut primitives: Vec<Primitive> = chunk_positions
            .iter()
            .enumerate()
            .map(|(i, pos)| {
                let min = Point3::new(
                    pos.x as f32 * chunk_size,
                    pos.y as f32 * chunk_size,
                    pos.z as f32 * chunk_size,
                );
                let max = min + Vector3::new(chunk_size, chunk_size, chunk_size);
                let center = Point3::new(
                    (min.x + max.x) * 0.5,
                    (min.y + max.y) * 0.5,
                    (min.z + max.z) * 0.5,
                );

                Primitive {
                    center,
                    aabb_min: min,
                    aabb_max: max,
                    index: i as u32,
                }
            })
            .collect();

        self.primitive_count = primitives.len() as u32;

        // Build BVH using SAH (Surface Area Heuristic)
        let mut nodes = Vec::new();
        let mut primitive_indices = Vec::new();
        self.max_depth = 0;

        let primitives_len = primitives.len();
        self.build_recursive(
            &mut nodes,
            &mut primitive_indices,
            &mut primitives,
            0,
            primitives_len,
            0,
        );

        self.node_count = nodes.len() as u32;

        // Upload to GPU
        queue.write_buffer(&self.node_buffer, 0, bytemuck::cast_slice(&nodes));
        queue.write_buffer(
            &self.primitive_buffer,
            0,
            bytemuck::cast_slice(&primitive_indices),
        );
    }

    /// Recursive BVH construction
    fn build_recursive(
        &mut self,
        nodes: &mut Vec<BvhNode>,
        primitive_indices: &mut Vec<u32>,
        primitives: &mut [Primitive],
        start: usize,
        end: usize,
        depth: u32,
    ) -> u32 {
        self.max_depth = self.max_depth.max(depth);

        let node_index = nodes.len() as u32;
        nodes.push(BvhNode {
            aabb_min: [0.0; 3],
            aabb_max: [0.0; 3],
            left_first: 0,
            prim_count: 0,
        });

        // Calculate bounds for this node
        let mut aabb_min = Point3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut aabb_max = Point3::new(f32::MIN, f32::MIN, f32::MIN);

        for i in start..end {
            aabb_min = Point3::new(
                aabb_min.x.min(primitives[i].aabb_min.x),
                aabb_min.y.min(primitives[i].aabb_min.y),
                aabb_min.z.min(primitives[i].aabb_min.z),
            );
            aabb_max = Point3::new(
                aabb_max.x.max(primitives[i].aabb_max.x),
                aabb_max.y.max(primitives[i].aabb_max.y),
                aabb_max.z.max(primitives[i].aabb_max.z),
            );
        }

        let prim_count = end - start;

        // Leaf node threshold
        if prim_count <= 4 || depth > 20 {
            // Create leaf node
            nodes[node_index as usize].aabb_min = aabb_min.into();
            nodes[node_index as usize].aabb_max = aabb_max.into();
            nodes[node_index as usize].left_first = primitive_indices.len() as u32;
            nodes[node_index as usize].prim_count = prim_count as u32;

            // Add primitive indices
            for i in start..end {
                primitive_indices.push(primitives[i].index);
            }

            return node_index;
        }

        // Find best split using SAH
        let (split_axis, split_pos) =
            self.find_best_split(&primitives[start..end], &aabb_min, &aabb_max);

        // Partition primitives
        let mut left_count = 0;
        for i in start..end {
            if primitives[i].center[split_axis] < split_pos {
                primitives.swap(start + left_count, i);
                left_count += 1;
            }
        }
        let mid = start + left_count;

        // Handle degenerate case
        let mid = if mid == start || mid == end {
            (start + end) / 2
        } else {
            mid
        };

        // Build children
        let left_child =
            self.build_recursive(nodes, primitive_indices, primitives, start, mid, depth + 1);
        let right_child =
            self.build_recursive(nodes, primitive_indices, primitives, mid, end, depth + 1);

        // Update node
        nodes[node_index as usize].aabb_min = aabb_min.into();
        nodes[node_index as usize].aabb_max = aabb_max.into();
        nodes[node_index as usize].left_first = left_child;
        nodes[node_index as usize].prim_count = 0;

        node_index
    }

    /// Find best split using Surface Area Heuristic
    fn find_best_split(
        &self,
        primitives: &[Primitive],
        aabb_min: &Point3<f32>,
        aabb_max: &Point3<f32>,
    ) -> (usize, f32) {
        let mut best_axis = 0;
        let mut best_pos = 0.0;
        let mut best_cost = f32::MAX;

        let parent_area = self.surface_area(aabb_min, aabb_max);

        // Try each axis
        for axis in 0..3 {
            // Sort primitives by center along axis
            let mut centers: Vec<f32> = primitives.iter().map(|p| p.center[axis]).collect();
            centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            // Try split positions
            let samples = centers.len().min(32);
            for i in 1..samples {
                let split_pos = centers[i * centers.len() / samples];

                // Calculate SAH cost
                let mut left_min = Point3::new(f32::MAX, f32::MAX, f32::MAX);
                let mut left_max = Point3::new(f32::MIN, f32::MIN, f32::MIN);
                let mut right_min = Point3::new(f32::MAX, f32::MAX, f32::MAX);
                let mut right_max = Point3::new(f32::MIN, f32::MIN, f32::MIN);
                let mut left_count = 0;
                let mut right_count = 0;

                for prim in primitives {
                    if prim.center[axis] < split_pos {
                        left_min = Point3::new(
                            left_min.x.min(prim.aabb_min.x),
                            left_min.y.min(prim.aabb_min.y),
                            left_min.z.min(prim.aabb_min.z),
                        );
                        left_max = Point3::new(
                            left_max.x.max(prim.aabb_max.x),
                            left_max.y.max(prim.aabb_max.y),
                            left_max.z.max(prim.aabb_max.z),
                        );
                        left_count += 1;
                    } else {
                        right_min = Point3::new(
                            right_min.x.min(prim.aabb_min.x),
                            right_min.y.min(prim.aabb_min.y),
                            right_min.z.min(prim.aabb_min.z),
                        );
                        right_max = Point3::new(
                            right_max.x.max(prim.aabb_max.x),
                            right_max.y.max(prim.aabb_max.y),
                            right_max.z.max(prim.aabb_max.z),
                        );
                        right_count += 1;
                    }
                }

                if left_count > 0 && right_count > 0 {
                    let left_area = self.surface_area(&left_min, &left_max);
                    let right_area = self.surface_area(&right_min, &right_max);

                    let cost = 1.0
                        + (left_area * left_count as f32 + right_area * right_count as f32)
                            / parent_area;

                    if cost < best_cost {
                        best_cost = cost;
                        best_axis = axis;
                        best_pos = split_pos;
                    }
                }
            }
        }

        (best_axis, best_pos)
    }

    /// Calculate surface area of AABB
    fn surface_area(&self, min: &Point3<f32>, max: &Point3<f32>) -> f32 {
        let d = max - min;
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    /// Get GPU buffers
    pub fn node_buffer(&self) -> &Buffer {
        &self.node_buffer
    }

    pub fn primitive_buffer(&self) -> &Buffer {
        &self.primitive_buffer
    }

    /// Get BVH statistics
    pub fn get_stats(&self) -> BvhStats {
        BvhStats {
            node_count: self.node_count,
            primitive_count: self.primitive_count,
            max_depth: self.max_depth,
            memory_usage_mb: (self.node_count as f32 * std::mem::size_of::<BvhNode>() as f32
                + self.primitive_count as f32 * 4.0)
                / (1024.0 * 1024.0),
        }
    }
}

/// BVH statistics
#[derive(Debug, Clone)]
pub struct BvhStats {
    pub node_count: u32,
    pub primitive_count: u32,
    pub max_depth: u32,
    pub memory_usage_mb: f32,
}
