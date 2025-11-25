/// Instance Query System
///
/// Efficient querying of instances by various criteria.
/// Uses bitsets and indices for fast filtering.
/// Supports complex queries with minimal allocations.
use crate::instance::{InstanceData, InstanceManagerData, InstanceId, InstanceType, MetadataStore, MetadataValue};
use bit_vec::BitVec;

/// Query filter conditions
#[derive(Debug, Clone)]
pub enum QueryFilter {
    /// Filter by instance type
    Type(InstanceType),
    /// Filter by multiple types
    TypeIn(Vec<InstanceType>),
    /// Filter by active status
    Active(bool),
    /// Filter by creation time range
    CreatedBetween(u64, u64),
    /// Filter by creator
    CreatedBy(InstanceId),
    /// Filter by metadata existence
    HasMetadata(&'static str),
    /// Filter by metadata value
    MetadataEquals(&'static str, MetadataValue),
    /// Filter by metadata range (numbers only)
    MetadataRange(&'static str, f64, f64),
    /// Combine filters with AND
    And(Box<QueryFilter>, Box<QueryFilter>),
    /// Combine filters with OR
    Or(Box<QueryFilter>, Box<QueryFilter>),
    /// Negate a filter
    Not(Box<QueryFilter>),
}

/// Query builder for fluent API
pub struct InstanceQuery {
    filters: Vec<QueryFilter>,
}

impl InstanceQuery {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Filter by type
    pub fn with_type(mut self, instance_type: InstanceType) -> Self {
        self.filters.push(QueryFilter::Type(instance_type));
        self
    }

    /// Filter by multiple types
    pub fn with_types(mut self, types: Vec<InstanceType>) -> Self {
        self.filters.push(QueryFilter::TypeIn(types));
        self
    }

    /// Only active instances
    pub fn active(mut self) -> Self {
        self.filters.push(QueryFilter::Active(true));
        self
    }

    /// Only inactive instances
    pub fn inactive(mut self) -> Self {
        self.filters.push(QueryFilter::Active(false));
        self
    }

    /// Created in time range
    pub fn created_between(mut self, start: u64, end: u64) -> Self {
        self.filters.push(QueryFilter::CreatedBetween(start, end));
        self
    }

    /// Created by specific actor
    pub fn created_by(mut self, creator: InstanceId) -> Self {
        self.filters.push(QueryFilter::CreatedBy(creator));
        self
    }

    /// Has specific metadata key
    pub fn has_metadata(mut self, key: &'static str) -> Self {
        self.filters.push(QueryFilter::HasMetadata(key));
        self
    }

    /// Metadata equals value
    pub fn metadata_equals(mut self, key: &'static str, value: MetadataValue) -> Self {
        self.filters.push(QueryFilter::MetadataEquals(key, value));
        self
    }

    /// Build final filter
    pub fn build(self) -> Option<QueryFilter> {
        if self.filters.is_empty() {
            None
        } else if self.filters.len() == 1 {
            self.filters.into_iter().next()
        } else {
            // Combine all with AND
            let mut iter = self.filters.into_iter();
            iter.next().map(|first| {
                iter.fold(first, |acc, filter| {
                    QueryFilter::And(Box::new(acc), Box::new(filter))
                })
            })
        }
    }
}

/// Query result with indices
pub struct QueryResult {
    /// Matching instance indices
    pub indices: Vec<usize>,
    /// Total instances checked
    pub total_checked: usize,
    /// Query execution time
    pub execution_time_us: u64,
}

/// Query executor
pub struct QueryExecutor<'a> {
    data: &'a InstanceManagerData,
    metadata: &'a MetadataStore,
}

impl<'a> QueryExecutor<'a> {
    pub fn new(data: &'a InstanceManagerData, metadata: &'a MetadataStore) -> Self {
        Self { data, metadata }
    }

    /// Execute a query
    pub fn execute(&self, filter: Option<&QueryFilter>) -> QueryResult {
        let start = std::time::Instant::now();

        let total = self.data.ids.len();
        let mut matches = BitVec::from_elem(total, true);

        // Apply filter if provided
        if let Some(f) = filter {
            self.apply_filter(f, &mut matches);
        }

        // Collect matching indices
        let indices: Vec<usize> = matches
            .iter()
            .enumerate()
            .filter_map(|(i, matches)| if matches { Some(i) } else { None })
            .collect();

        QueryResult {
            indices,
            total_checked: total,
            execution_time_us: start.elapsed().as_micros() as u64,
        }
    }

    /// Apply filter to bitset
    fn apply_filter(&self, filter: &QueryFilter, matches: &mut BitVec) {
        match filter {
            QueryFilter::Type(t) => {
                for (i, instance_type) in self.data.types.iter().enumerate() {
                    if instance_type != t {
                        matches.set(i, false);
                    }
                }
            }

            QueryFilter::TypeIn(types) => {
                for (i, instance_type) in self.data.types.iter().enumerate() {
                    if !types.contains(instance_type) {
                        matches.set(i, false);
                    }
                }
            }

            QueryFilter::Active(active) => {
                for (i, is_active) in self.data.active.iter().enumerate() {
                    if is_active != active {
                        matches.set(i, false);
                    }
                }
            }

            QueryFilter::CreatedBetween(start, end) => {
                for (i, created) in self.data.created_at.iter().enumerate() {
                    if created < start || created > end {
                        matches.set(i, false);
                    }
                }
            }

            QueryFilter::CreatedBy(creator) => {
                for (i, created_by) in self.data.created_by.iter().enumerate() {
                    if created_by != creator {
                        matches.set(i, false);
                    }
                }
            }

            QueryFilter::HasMetadata(key) => {
                for i in 0..self.data.ids.len() {
                    if matches[i] {
                        let id = &self.data.ids[i];
                        if self.metadata.get(id, key).is_none() {
                            matches.set(i, false);
                        }
                    }
                }
            }

            QueryFilter::MetadataEquals(key, value) => {
                for i in 0..self.data.ids.len() {
                    if matches[i] {
                        let id = &self.data.ids[i];
                        match self.metadata.get(id, key) {
                            Some(v) if &v == value => {}
                            _ => matches.set(i, false),
                        }
                    }
                }
            }

            QueryFilter::MetadataRange(key, min, max) => {
                for i in 0..self.data.ids.len() {
                    if matches[i] {
                        let id = &self.data.ids[i];
                        let in_range = match self.metadata.get(id, key) {
                            Some(MetadataValue::F32(v)) => (*min <= v as f64) && (v as f64 <= *max),
                            Some(MetadataValue::F64(v)) => (*min <= v) && (v <= *max),
                            Some(MetadataValue::I32(v)) => {
                                (*min <= v as f64) && ((v as f64) <= *max)
                            }
                            Some(MetadataValue::I64(v)) => {
                                (*min <= v as f64) && ((v as f64) <= *max)
                            }
                            _ => false,
                        };
                        if !in_range {
                            matches.set(i, false);
                        }
                    }
                }
            }

            QueryFilter::And(a, b) => {
                self.apply_filter(a, matches);
                self.apply_filter(b, matches);
            }

            QueryFilter::Or(a, b) => {
                let mut matches_a = matches.clone();
                let mut matches_b = matches.clone();
                self.apply_filter(a, &mut matches_a);
                self.apply_filter(b, &mut matches_b);

                // OR operation
                for i in 0..matches.len() {
                    matches.set(i, matches_a[i] || matches_b[i]);
                }
            }

            QueryFilter::Not(f) => {
                let mut inverted = BitVec::from_elem(matches.len(), true);
                self.apply_filter(f, &mut inverted);

                // AND with NOT
                for i in 0..matches.len() {
                    matches.set(i, matches[i] && !inverted[i]);
                }
            }
        }
    }

    /// Count instances matching filter (without collecting indices)
    pub fn count(&self, filter: Option<&QueryFilter>) -> usize {
        let total = self.data.ids.len();
        let mut matches = BitVec::from_elem(total, true);

        if let Some(f) = filter {
            self.apply_filter(f, &mut matches);
        }

        matches.iter().filter(|&m| m).count()
    }
}

/// Pre-built indices for common queries
pub struct QueryIndices {
    /// Instances by type
    by_type: std::collections::HashMap<InstanceType, Vec<usize>>,
    /// Active instances
    active: Vec<usize>,
    /// Instances by creator
    by_creator: std::collections::HashMap<InstanceId, Vec<usize>>,
}

impl QueryIndices {
    pub fn build(data: &InstanceData) -> Self {
        let mut by_type = std::collections::HashMap::new();
        let mut active = Vec::new();
        let mut by_creator = std::collections::HashMap::new();

        for (i, id) in data.ids.iter().enumerate() {
            // By type
            by_type
                .entry(data.types[i])
                .or_insert_with(Vec::new)
                .push(i);

            // Active
            if data.active[i] {
                active.push(i);
            }

            // By creator
            by_creator
                .entry(data.created_by[i])
                .or_insert_with(Vec::new)
                .push(i);
        }

        Self {
            by_type,
            active,
            by_creator,
        }
    }

    /// Get indices for type
    pub fn get_by_type(&self, t: InstanceType) -> &[usize] {
        self.by_type.get(&t).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get active indices
    pub fn get_active(&self) -> &[usize] {
        &self.active
    }

    /// Get indices by creator
    pub fn get_by_creator(&self, creator: &InstanceId) -> &[usize] {
        self.by_creator
            .get(creator)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = InstanceQuery::new()
            .with_type(InstanceType::Item)
            .active()
            .has_metadata("name")
            .build();

        assert!(query.is_some());
    }

    #[test]
    fn test_query_execution() {
        let mut data = InstanceData::new();
        let mut metadata = MetadataStore::new();

        // Add test instances
        let creator = InstanceId::new();
        let id1 = InstanceId::new();
        let id2 = InstanceId::new();

        data.add(id1, InstanceType::Item, creator)
            .expect("Failed to add item instance");
        data.add(id2, InstanceType::Block, creator)
            .expect("Failed to add block instance");

        metadata
            .set(id1, "name", MetadataValue::String("Sword".to_string()))
            .expect("Failed to set metadata");

        // Execute query
        let executor = QueryExecutor::new(&data, &metadata);
        let filter = QueryFilter::And(
            Box::new(QueryFilter::Type(InstanceType::Item)),
            Box::new(QueryFilter::HasMetadata("name")),
        );

        let result = executor.execute(Some(&filter));
        assert_eq!(result.indices.len(), 1);
        assert_eq!(result.indices[0], 0);
    }
}
