//! Plugin system performance benchmarks
//!
//! Tests plugin system performance impact on server tick rate and overall performance

use super::*;
use mirai_core::{App, World, Instance, Plugin};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Plugin performance benchmark suite
pub struct PluginPerformanceBenchmarks {
    runner: BenchmarkRunner,
    rt: Runtime,
}

impl PluginPerformanceBenchmarks {
    pub fn new() -> Self {
        Self {
            runner: BenchmarkRunner::new(BenchmarkConfig::default()),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Run all plugin performance benchmarks
    pub fn run_all_benchmarks(&mut self) -> Vec<BenchmarkResult> {
        println!("Running plugin performance benchmarks...");

        self.benchmark_plugin_loading_performance();
        self.benchmark_tick_rate_impact();
        self.benchmark_plugin_system_overhead();
        self.benchmark_plugin_event_handling();
        self.benchmark_plugin_scalability();

        self.runner.results().to_vec()
    }

    /// Benchmark plugin loading and initialization performance
    fn benchmark_plugin_loading_performance(&mut self) {
        println!("Benchmarking plugin loading performance...");

        // Benchmark server startup without plugins
        let baseline_startup = self.runner.benchmark("server_startup_no_plugins", || {
            self.rt.block_on(async {
                let app = App::new();
                let instance = app.build_instance().await
                    .expect("Failed to build instance");
                instance
            })
        });

        // Benchmark server startup with single plugin
        let single_plugin_startup = self.runner.benchmark("server_startup_single_plugin", || {
            self.rt.block_on(async {
                let mut app = App::new();
                app.add_plugin(LightweightTestPlugin);
                let instance = app.build_instance().await
                    .expect("Failed to build instance");
                instance
            })
        });

        // Benchmark server startup with multiple plugins
        let multiple_plugins_startup = self.runner.benchmark("server_startup_multiple_plugins", || {
            self.rt.block_on(async {
                let mut app = App::new();
                app.add_plugin(LightweightTestPlugin);
                app.add_plugin(MediumWeightTestPlugin);
                app.add_plugin(HeavyWeightTestPlugin);
                app.add_plugin(EventTestPlugin);
                app.add_plugin(SystemTestPlugin);
                let instance = app.build_instance().await
                    .expect("Failed to build instance");
                instance
            })
        });

        // Calculate plugin loading overhead
        let single_plugin_overhead = single_plugin_startup.as_nanos() as f64 / baseline_startup.as_nanos() as f64;
        let multiple_plugins_overhead = multiple_plugins_startup.as_nanos() as f64 / baseline_startup.as_nanos() as f64;

        self.runner.results.push(BenchmarkResult {
            name: "plugin_loading_single_overhead".to_string(),
            value: single_plugin_overhead,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("plugin_loading_overhead"),
        });

        self.runner.results.push(BenchmarkResult {
            name: "plugin_loading_multiple_overhead".to_string(),
            value: multiple_plugins_overhead,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("plugin_loading_overhead"),
        });

        // Benchmark individual plugin initialization
        self.runner.benchmark("plugin_initialization_lightweight", || {
            let plugin = LightweightTestPlugin;
            let mut app = App::new();
            plugin.build(&mut app);
        });

        self.runner.benchmark("plugin_initialization_heavyweight", || {
            let plugin = HeavyWeightTestPlugin;
            let mut app = App::new();
            plugin.build(&mut app);
        });
    }

    /// Benchmark plugin impact on server tick rate
    fn benchmark_tick_rate_impact(&mut self) {
        println!("Benchmarking plugin impact on tick rate...");

        // Baseline tick rate without plugins
        let baseline_instance = self.rt.block_on(async {
            let app = App::new();
            app.build_instance().await.expect("Failed to build instance")
        });

        let baseline_tick_time = self.runner.benchmark("tick_rate_baseline", || {
            let world = baseline_instance.world();
            world.run_systems();
        });

        // Tick rate with lightweight plugins
        let lightweight_instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(LightweightTestPlugin);
            app.add_plugin(EventTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let lightweight_tick_time = self.runner.benchmark("tick_rate_lightweight_plugins", || {
            let world = lightweight_instance.world();
            world.run_systems();
        });

        // Tick rate with heavy plugins
        let heavy_instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(HeavyWeightTestPlugin);
            app.add_plugin(SystemTestPlugin);
            app.add_plugin(MediumWeightTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let heavy_tick_time = self.runner.benchmark("tick_rate_heavy_plugins", || {
            let world = heavy_instance.world();
            world.run_systems();
        });

        // Calculate tick rate impact
        let lightweight_impact = lightweight_tick_time.as_nanos() as f64 / baseline_tick_time.as_nanos() as f64;
        let heavy_impact = heavy_tick_time.as_nanos() as f64 / baseline_tick_time.as_nanos() as f64;

        self.runner.results.push(BenchmarkResult {
            name: "tick_rate_lightweight_impact".to_string(),
            value: lightweight_impact,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("tick_rate_impact"),
        });

        self.runner.results.push(BenchmarkResult {
            name: "tick_rate_heavy_impact".to_string(),
            value: heavy_impact,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("tick_rate_impact"),
        });

        // Benchmark sustained tick rate performance
        let tick_count = 1000;
        let start = Instant::now();
        
        for _ in 0..tick_count {
            heavy_instance.world().run_systems();
        }
        
        let sustained_time = start.elapsed();
        let ticks_per_second = tick_count as f64 / sustained_time.as_secs_f64();

        self.runner.results.push(BenchmarkResult {
            name: "sustained_tick_rate_with_plugins".to_string(),
            value: ticks_per_second,
            unit: "ticks/sec".to_string(),
            lower_is_better: false,
            metadata: self.runner.create_metadata("sustained_tick_rate"),
        });
    }

    /// Benchmark plugin system execution overhead
    fn benchmark_plugin_system_overhead(&mut self) {
        println!("Benchmarking plugin system overhead...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(SystemTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Create entities for systems to process
        for i in 0..1000 {
            let entity = world.spawn_entity();
            world.add_component(entity, TestComponent {
                id: i,
                value: i as f32,
                data: format!("entity_{}", i),
            });
        }

        // Benchmark individual system execution
        self.runner.benchmark("plugin_system_execution", || {
            test_processing_system(world);
        });

        // Benchmark system with entity iteration
        self.runner.benchmark("plugin_system_entity_iteration", || {
            entity_iteration_system(world);
        });

        // Benchmark system with complex operations
        self.runner.benchmark("plugin_system_complex_operations", || {
            complex_operations_system(world);
        });

        // Benchmark system scheduling overhead
        self.runner.benchmark("plugin_system_scheduling", || {
            world.run_systems();
        });
    }

    /// Benchmark plugin event handling performance
    fn benchmark_plugin_event_handling(&mut self) {
        println!("Benchmarking plugin event handling...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(EventTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Benchmark event creation and dispatch
        self.runner.benchmark("plugin_event_dispatch", || {
            // Simulate event creation and handling
            let event = TestEvent {
                id: 1,
                data: "benchmark_event".to_string(),
                timestamp: Instant::now(),
            };
            
            // Process event through plugin systems
            world.run_systems();
            event
        });

        // Benchmark event throughput
        let event_count = 1000;
        let start = Instant::now();
        
        for i in 0..event_count {
            let _event = TestEvent {
                id: i,
                data: format!("event_{}", i),
                timestamp: Instant::now(),
            };
            
            // Process through systems
            world.run_systems();
        }
        
        let event_time = start.elapsed();
        let events_per_second = event_count as f64 / event_time.as_secs_f64();

        self.runner.results.push(BenchmarkResult {
            name: "plugin_event_throughput".to_string(),
            value: events_per_second,
            unit: "events/sec".to_string(),
            lower_is_better: false,
            metadata: self.runner.create_metadata("plugin_event_throughput"),
        });
    }

    /// Benchmark plugin system scalability
    fn benchmark_plugin_scalability(&mut self) {
        println!("Benchmarking plugin scalability...");

        // Test with increasing number of plugins
        let plugin_counts = vec![1, 5, 10, 20];
        
        for count in plugin_counts {
            let instance = self.rt.block_on(async {
                let mut app = App::new();
                
                // Add multiple instances of test plugins
                for i in 0..count {
                    app.add_plugin(ScalabilityTestPlugin::new(i));
                }
                
                app.build_instance().await.expect("Failed to build instance")
            });

            let world = instance.world();

            // Create entities for processing
            for i in 0..1000 {
                let entity = world.spawn_entity();
                world.add_component(entity, TestComponent {
                    id: i,
                    value: i as f32,
                    data: format!("scalability_test_{}", i),
                });
            }

            // Benchmark system execution with multiple plugins
            let execution_time = self.runner.benchmark(&format!("plugin_scalability_{}_plugins", count), || {
                world.run_systems();
            });

            // Record scalability metrics
            self.runner.results.push(BenchmarkResult {
                name: format!("plugin_scalability_{}_plugins_time", count),
                value: execution_time.as_secs_f64(),
                unit: "seconds".to_string(),
                lower_is_better: true,
                metadata: self.runner.create_metadata("plugin_scalability"),
            });
        }

        // Test with increasing entity counts
        let entity_counts = vec![100, 1000, 5000, 10000];
        
        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(SystemTestPlugin);
            app.add_plugin(EventTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        for count in entity_counts {
            let world = instance.world();
            
            // Clear previous entities
            for entity in world.query_entities() {
                world.despawn_entity(entity);
            }

            // Create entities
            for i in 0..count {
                let entity = world.spawn_entity();
                world.add_component(entity, TestComponent {
                    id: i,
                    value: i as f32,
                    data: format!("entity_{}", i),
                });
            }

            // Benchmark processing with different entity counts
            let processing_time = self.runner.benchmark(&format!("plugin_entity_scalability_{}_entities", count), || {
                world.run_systems();
            });

            // Calculate entities per second
            let entities_per_second = count as f64 / processing_time.as_secs_f64();
            
            self.runner.results.push(BenchmarkResult {
                name: format!("plugin_entity_processing_rate_{}_entities", count),
                value: entities_per_second,
                unit: "entities/sec".to_string(),
                lower_is_better: false,
                metadata: self.runner.create_metadata("plugin_entity_scalability"),
            });
        }
    }

    /// Save benchmark results
    pub fn save_results(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.runner.save_results(path)
    }
}

// Test plugins for benchmarking

struct LightweightTestPlugin;

impl Plugin for LightweightTestPlugin {
    fn name(&self) -> &'static str {
        "lightweight_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.add_system(lightweight_system);
    }
}

struct MediumWeightTestPlugin;

impl Plugin for MediumWeightTestPlugin {
    fn name(&self) -> &'static str {
        "medium_weight_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.add_system(medium_weight_system);
    }
}

struct HeavyWeightTestPlugin;

impl Plugin for HeavyWeightTestPlugin {
    fn name(&self) -> &'static str {
        "heavy_weight_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.add_system(heavy_weight_system);
    }
}

struct EventTestPlugin;

impl Plugin for EventTestPlugin {
    fn name(&self) -> &'static str {
        "event_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.add_system(event_handling_system);
    }
}

struct SystemTestPlugin;

impl Plugin for SystemTestPlugin {
    fn name(&self) -> &'static str {
        "system_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.add_system(test_processing_system);
        app.add_system(entity_iteration_system);
        app.add_system(complex_operations_system);
    }
}

struct ScalabilityTestPlugin {
    id: usize,
}

impl ScalabilityTestPlugin {
    fn new(id: usize) -> Self {
        Self { id }
    }
}

impl Plugin for ScalabilityTestPlugin {
    fn name(&self) -> &'static str {
        "scalability_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.add_system(scalability_system);
    }
}

// Test components and events

#[derive(Debug, Clone)]
struct TestComponent {
    id: usize,
    value: f32,
    data: String,
}

impl mirai_core::Component for TestComponent {}

#[derive(Debug, Clone)]
struct TestEvent {
    id: usize,
    data: String,
    timestamp: Instant,
}

// Test systems

fn lightweight_system(world: &mut World) {
    // Minimal processing
    for component in world.query::<&TestComponent>() {
        let _id = component.id;
    }
}

fn medium_weight_system(world: &mut World) {
    // Moderate processing
    for mut component in world.query::<&mut TestComponent>() {
        component.value += 1.0;
        if component.value > 1000.0 {
            component.value = 0.0;
        }
    }
}

fn heavy_weight_system(world: &mut World) {
    // Heavy processing
    for mut component in world.query::<&mut TestComponent>() {
        // Simulate expensive computation
        let mut sum = 0.0;
        for i in 0..100 {
            sum += (component.id + i) as f32 * component.value;
        }
        component.value = sum % 1000.0;
        component.data = format!("processed_{}_{}", component.id, sum as usize);
    }
}

fn test_processing_system(world: &mut World) {
    for mut component in world.query::<&mut TestComponent>() {
        component.value += 0.1;
        component.data = format!("processed_{}", component.id);
    }
}

fn entity_iteration_system(world: &mut World) {
    let mut count = 0;
    for component in world.query::<&TestComponent>() {
        count += component.id;
    }
    // Use count to prevent optimization
    let _result = count % 1000;
}

fn complex_operations_system(world: &mut World) {
    for mut component in world.query::<&mut TestComponent>() {
        // Simulate complex operations
        let hash = component.data.len() * component.id;
        component.value = (hash % 100) as f32;
        
        if component.value > 50.0 {
            component.data = format!("high_value_{}", component.id);
        } else {
            component.data = format!("low_value_{}", component.id);
        }
    }
}

fn event_handling_system(world: &mut World) {
    // Simulate event processing
    for component in world.query::<&TestComponent>() {
        if component.id % 10 == 0 {
            // Simulate event creation
            let _event = TestEvent {
                id: component.id,
                data: component.data.clone(),
                timestamp: Instant::now(),
            };
        }
    }
}

fn scalability_system(world: &mut World) {
    // System that scales with entity count
    for mut component in world.query::<&mut TestComponent>() {
        component.value = (component.value + 1.0) % 100.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_performance_benchmarks() {
        let mut benchmarks = PluginPerformanceBenchmarks::new();
        let results = benchmarks.run_all_benchmarks();
        
        assert!(!results.is_empty());
        
        // Verify we have plugin-specific benchmarks
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("plugin")));
        assert!(names.iter().any(|n| n.contains("tick_rate")));
        assert!(names.iter().any(|n| n.contains("scalability")));
    }

    #[test]
    fn test_plugin_loading_overhead_acceptable() {
        let mut benchmarks = PluginPerformanceBenchmarks::new();
        benchmarks.benchmark_plugin_loading_performance();
        
        let results = benchmarks.runner.results();
        let overhead_result = results.iter()
            .find(|r| r.name == "plugin_loading_single_overhead")
            .expect("Should have plugin loading overhead measurement");
        
        // Plugin loading overhead should be reasonable (less than 100% increase)
        assert!(overhead_result.value < 2.0, 
                "Plugin loading overhead too high: {}x", overhead_result.value);
    }

    #[test]
    fn test_tick_rate_impact_acceptable() {
        let mut benchmarks = PluginPerformanceBenchmarks::new();
        benchmarks.benchmark_tick_rate_impact();
        
        let results = benchmarks.runner.results();
        let impact_result = results.iter()
            .find(|r| r.name == "tick_rate_lightweight_impact")
            .expect("Should have tick rate impact measurement");
        
        // Lightweight plugins should have minimal impact (less than 20% increase)
        assert!(impact_result.value < 1.2, 
                "Lightweight plugin tick rate impact too high: {}x", impact_result.value);
    }

    #[test]
    fn test_sustained_tick_rate_performance() {
        let mut benchmarks = PluginPerformanceBenchmarks::new();
        benchmarks.benchmark_tick_rate_impact();
        
        let results = benchmarks.runner.results();
        let tick_rate_result = results.iter()
            .find(|r| r.name == "sustained_tick_rate_with_plugins")
            .expect("Should have sustained tick rate measurement");
        
        // Should maintain reasonable tick rate (at least 20 TPS)
        assert!(tick_rate_result.value >= 20.0, 
                "Sustained tick rate too low: {} TPS", tick_rate_result.value);
    }
}