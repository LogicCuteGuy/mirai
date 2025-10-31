//! System execution and scheduling for the ECS

use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Trait for systems that can be executed on the world
pub trait System: Send + Sync {
    /// Execute the system
    fn run(&mut self, world: &mut super::World) -> Result<()>;
    
    /// Get the system name for debugging
    fn name(&self) -> &str;
    
    /// Check if the system should run this frame
    fn should_run(&self) -> bool {
        true
    }
    
    /// Get system dependencies (systems that must run before this one)
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }
    
    /// Check if this system can run in parallel with others
    fn is_parallel_safe(&self) -> bool {
        false
    }
}

/// System metadata for scheduling
#[derive(Debug, Clone)]
struct SystemInfo {
    name: String,
    dependencies: Vec<String>,
    is_parallel_safe: bool,
}

/// Execution stage for grouping systems
#[derive(Debug, Clone)]
pub struct ExecutionStage {
    name: String,
    systems: Vec<usize>,
    parallel_groups: Vec<Vec<usize>>,
}

impl ExecutionStage {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            systems: Vec::new(),
            parallel_groups: Vec::new(),
        }
    }
    
    pub fn add_system(&mut self, system_index: usize) {
        self.systems.push(system_index);
    }
}

/// Manages and schedules system execution with parallel support
pub struct SystemScheduler {
    systems: Vec<Box<dyn System>>,
    system_info: Vec<SystemInfo>,
    stages: Vec<ExecutionStage>,
    system_name_to_index: HashMap<String, usize>,
}

impl SystemScheduler {
    /// Create a new system scheduler
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            system_info: Vec::new(),
            stages: Vec::new(),
            system_name_to_index: HashMap::new(),
        }
    }
    
    /// Add a system to the scheduler
    pub fn add_system<S: System + 'static>(&mut self, system: S) -> &mut Self {
        let index = self.systems.len();
        let name = system.name().to_string();
        let dependencies = system.dependencies();
        let is_parallel_safe = system.is_parallel_safe();
        
        // Check for duplicate system names
        if self.system_name_to_index.contains_key(&name) {
            tracing::warn!("System '{}' already exists, replacing", name);
        }
        
        self.system_name_to_index.insert(name.clone(), index);
        self.system_info.push(SystemInfo {
            name: name.clone(),
            dependencies,
            is_parallel_safe,
        });
        self.systems.push(Box::new(system));
        
        self
    }
    
    /// Add a system to a specific stage
    pub fn add_system_to_stage<S: System + 'static>(&mut self, system: S, stage_name: &str) -> &mut Self {
        self.add_system(system);
        let system_index = self.systems.len() - 1;
        
        // Find or create the stage
        let stage_index = self.stages.iter().position(|s| s.name == stage_name)
            .unwrap_or_else(|| {
                self.stages.push(ExecutionStage::new(stage_name));
                self.stages.len() - 1
            });
        
        self.stages[stage_index].add_system(system_index);
        self
    }
    
    /// Build execution order based on dependencies
    pub fn build_execution_order(&mut self) -> Result<()> {
        // Validate dependencies
        for info in &self.system_info {
            for dep in &info.dependencies {
                if !self.system_name_to_index.contains_key(dep) {
                    return Err(anyhow::anyhow!(
                        "System '{}' depends on '{}' which doesn't exist", info.name, dep
                    ));
                }
            }
        }
        
        // Build parallel groups for each stage
        let mut stage_systems = Vec::new();
        for stage in &self.stages {
            stage_systems.push(stage.systems.clone());
        }
        
        for (i, systems) in stage_systems.into_iter().enumerate() {
            self.stages[i].parallel_groups = self.build_parallel_groups(&systems)?;
        }
        
        Ok(())
    }
    
    /// Build parallel execution groups for a set of systems
    fn build_parallel_groups(&self, system_indices: &[usize]) -> Result<Vec<Vec<usize>>> {
        let mut groups = Vec::new();
        let mut remaining: HashSet<usize> = system_indices.iter().copied().collect();
        
        while !remaining.is_empty() {
            let mut current_group = Vec::new();
            let mut to_remove = Vec::new();
            
            for &system_index in &remaining {
                let info = &self.system_info[system_index];
                
                // Check if all dependencies are satisfied
                let deps_satisfied = info.dependencies.iter().all(|dep| {
                    let dep_index = self.system_name_to_index[dep];
                    !remaining.contains(&dep_index)
                });
                
                if deps_satisfied {
                    current_group.push(system_index);
                    to_remove.push(system_index);
                }
            }
            
            if current_group.is_empty() {
                return Err(anyhow::anyhow!("Circular dependency detected in systems"));
            }
            
            for index in to_remove {
                remaining.remove(&index);
            }
            
            groups.push(current_group);
        }
        
        Ok(groups)
    }
    
    /// Run all systems in order with parallel execution where possible
    pub fn run_systems(&mut self, world: &mut super::World) -> Result<()> {
        if self.stages.is_empty() {
            // If no stages defined, create a default stage with all systems
            let mut default_stage = ExecutionStage::new("default");
            for i in 0..self.systems.len() {
                default_stage.add_system(i);
            }
            self.stages.push(default_stage);
            self.build_execution_order()?;
        }
        
        // Clone stages to avoid borrowing issues
        let stages = self.stages.clone();
        for stage in &stages {
            self.run_stage(stage, world)?;
        }
        
        Ok(())
    }
    
    /// Run a single execution stage
    fn run_stage(&mut self, stage: &ExecutionStage, world: &mut super::World) -> Result<()> {
        tracing::trace!("Running stage: {}", stage.name);
        
        for group in &stage.parallel_groups {
            if group.len() == 1 {
                // Single system, run directly
                let system_index = group[0];
                self.run_single_system(system_index, world)?;
            } else {
                // Multiple systems, check if they can run in parallel
                let can_run_parallel = group.iter().all(|&i| self.system_info[i].is_parallel_safe);
                
                if can_run_parallel {
                    self.run_systems_parallel(group, world)?;
                } else {
                    // Run sequentially
                    for &system_index in group {
                        self.run_single_system(system_index, world)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Run a single system
    fn run_single_system(&mut self, system_index: usize, world: &mut super::World) -> Result<()> {
        let system = &mut self.systems[system_index];
        
        if system.should_run() {
            tracing::trace!("Running system: {}", system.name());
            
            if let Err(e) = system.run(world) {
                tracing::error!("System '{}' failed: {}", system.name(), e);
                return Err(anyhow::anyhow!("System '{}' failed: {}", system.name(), e));
            }
        }
        
        Ok(())
    }
    
    /// Run multiple systems in parallel (placeholder implementation)
    fn run_systems_parallel(&mut self, _group: &[usize], _world: &mut super::World) -> Result<()> {
        // Note: True parallel execution would require more sophisticated world access patterns
        // For now, we'll run them sequentially but this provides the framework for future enhancement
        tracing::trace!("Running {} systems in parallel group", _group.len());
        
        for &system_index in _group {
            self.run_single_system(system_index, _world)?;
        }
        
        Ok(())
    }
    
    /// Get the number of registered systems
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }
    
    /// Get the number of stages
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
    
    /// Clear all systems and stages
    pub fn clear(&mut self) {
        self.systems.clear();
        self.system_info.clear();
        self.stages.clear();
        self.system_name_to_index.clear();
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple function-based system implementation
pub struct FunctionSystem {
    name: String,
    function: Box<dyn Fn(&mut super::World) -> Result<()> + Send + Sync>,
    dependencies: Vec<String>,
    is_parallel_safe: bool,
}

impl FunctionSystem {
    /// Create a new function system
    pub fn new<F>(name: impl Into<String>, function: F) -> Self
    where
        F: Fn(&mut super::World) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            function: Box::new(function),
            dependencies: Vec::new(),
            is_parallel_safe: false,
        }
    }
    
    /// Create a new function system with dependencies
    pub fn with_dependencies<F>(
        name: impl Into<String>, 
        function: F, 
        dependencies: Vec<String>
    ) -> Self
    where
        F: Fn(&mut super::World) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            function: Box::new(function),
            dependencies,
            is_parallel_safe: false,
        }
    }
    
    /// Create a new parallel-safe function system
    pub fn parallel<F>(name: impl Into<String>, function: F) -> Self
    where
        F: Fn(&mut super::World) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            function: Box::new(function),
            dependencies: Vec::new(),
            is_parallel_safe: true,
        }
    }
}

impl System for FunctionSystem {
    fn run(&mut self, world: &mut super::World) -> Result<()> {
        (self.function)(world)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
    
    fn is_parallel_safe(&self) -> bool {
        self.is_parallel_safe
    }
}