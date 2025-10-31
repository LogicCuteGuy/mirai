//! Baseline comparison benchmarks
//!
//! Compares unified server performance against baseline Mirai implementation

use super::*;
use mirai_core::{App, World, PerformanceManager, Instance};
use mirai_proto::{RakNetConnectionManager, RakNetManagerConfig};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Baseline performance comparison suite
pub struct BaselineComparison {
    runner: BenchmarkRunner,
    rt: Runtime,
}

impl BaselineComparison {
    pub fn new() -> Self {
        Self {
            runner: BenchmarkRunner::new(BenchmarkConfig::default()),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Run all baseline comparison benchmarks
    pub fn run_all_benchmarks(&mut self) -> Vec<BenchmarkResult> {
        println!("Running baseline comparison benchmarks...");

        self.benchmark_server_startup();
        self.benchmark_entity_spawning();
        self.benchmark_component_operations();
        self.benchmark_system_execution();
        self.benchmark_packet_processing();
        self.benchmark_world_operations();
        self.benchmark_memory_allocation();

        self.runner.results().to_vec()
    }

    /// Benchmark server startup time
    fn benchmark_server_startup(&mut self) {
        println!("Benchmarking server startup...");

        self.runner.benchmark("server_startup", || {
            self.rt.block_on(async {
                let mut app = App::new();
                app.add_plugin(TestPlugin);
                
                let instance = app.build_instance().await
                    .expect("Failed to build instance");
                
                // Simulate minimal startup operations
                let _world = instance.world();
                let _performance = instance.performance_manager();
                
                // Return startup time marker
                Instant::now()
            })
        });
    }

    /// Benchmark entity spawning performance
    fn benchmark_entity_spawning(&mut self) {
        println!("Benchmarking entity spawning...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(TestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Benchmark single entity spawn
        self.runner.benchmark("entity_spawn_single", || {
            world.spawn_entity()
        });

        // Benchmark batch entity spawning
        self.runner.benchmark("entity_spawn_batch_1000", || {
            for _ in 0..1000 {
                world.spawn_entity();
            }
        });

        // Benchmark throughput
        self.runner.benchmark_throughput("entity_spawn_throughput", 10000, || {
            world.spawn_entity();
        });
    }

    /// Benchmark component operations
    fn benchmark_component_operations(&mut self) {
        println!("Benchmarking component operations...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(TestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Create entities for testing
        let mut entities = Vec::new();
        for i in 0..1000 {
            let entity = world.spawn_entity();
            world.add_component(entity, TestComponent { 
                id: i, 
                data: format!("test_{}", i) 
            });
            entities.push(entity);
        }

        // Benchmark component addition
        self.runner.benchmark("component_add", || {
            let entity = world.spawn_entity();
            world.add_component(entity, TestComponent { 
                id: 999, 
                data: "benchmark".to_string() 
            });
        });

        // Benchmark component access
        self.runner.benchmark("component_access", || {
            if let Some(entity) = entities.first() {
                let _component = world.get_component::<TestComponent>(*entity);
            }
        });

        // Benchmark component iteration
        self.runner.benchmark("component_iteration_1000", || {
            for _component in world.query::<&TestComponent>() {
                // Access component data
                let _id = _component.id;
            }
        });
    }

    /// Benchmark ECS system execution
    fn benchmark_system_execution(&mut self) {
        println!("Benchmarking system execution...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(TestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Create test entities
        for i in 0..5000 {
            let entity = world.spawn_entity();
            world.add_component(entity, TestComponent { 
                id: i, 
                data: format!("entity_{}", i) 
            });
            world.add_component(entity, PositionComponent { 
                x: i as f32, 
                y: 0.0, 
                z: 0.0 
            });
        }

        // Benchmark system execution
        self.runner.benchmark("system_execution_5000_entities", || {
            world.run_systems();
        });

        // Benchmark single system
        self.runner.benchmark("single_system_execution", || {
            test_system(world);
        });
    }

    /// Benchmark packet processing performance
    fn benchmark_packet_processing(&mut self) {
        println!("Benchmarking packet processing...");

        self.rt.block_on(async {
            let addr = std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 
                0
            );
            let config = RakNetManagerConfig::default();
            
            let manager = RakNetConnectionManager::new(addr, config).await
                .expect("Failed to create RakNet manager");
            
            manager.start().await.expect("Failed to start manager");

            // Benchmark packet creation
            self.runner.benchmark("packet_creation", || {
                mirai_proto::RawBedrockPacket {
                    id: 0x01,
                    data: bytes::Bytes::from("benchmark_packet"),
                    direction: mirai_proto::PacketDirection::ServerToClient,
                }
            });

            // Benchmark packet processing throughput
            self.runner.benchmark_throughput("packet_processing", 1000, || {
                let packet = mirai_proto::RawBedrockPacket {
                    id: 0x01,
                    data: bytes::Bytes::from("benchmark"),
                    direction: mirai_proto::PacketDirection::ServerToClient,
                };
                
                // Simulate packet processing
                let _processed = packet.id + 1;
            });

            manager.stop().await.expect("Failed to stop manager");
        });
    }

    /// Benchmark world operations
    fn benchmark_world_operations(&mut self) {
        println!("Benchmarking world operations...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(TestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Benchmark world queries
        self.runner.benchmark("world_query_empty", || {
            let _count = world.query::<&TestComponent>().count();
        });

        // Create entities for query benchmarks
        for i in 0..1000 {
            let entity = world.spawn_entity();
            world.add_component(entity, TestComponent { 
                id: i, 
                data: format!("query_test_{}", i) 
            });
        }

        self.runner.benchmark("world_query_1000", || {
            let _count = world.query::<&TestComponent>().count();
        });

        // Benchmark complex queries
        self.runner.benchmark("world_complex_query", || {
            for (_test, _pos) in world.query::<(&TestComponent, &PositionComponent)>() {
                // Process components
                let _result = _test.id + _pos.x as usize;
            }
        });
    }

    /// Benchmark memory allocation patterns
    fn benchmark_memory_allocation(&mut self) {
        println!("Benchmarking memory allocation...");

        let performance_manager = PerformanceManager::new();
        let memory_pools = performance_manager.memory_pools();

        // Benchmark pool allocation
        self.runner.benchmark("memory_pool_allocation", || {
            let _buffer = memory_pools.entity_pool.get();
        });

        // Benchmark pool reuse
        self.runner.benchmark("memory_pool_reuse", || {
            let buffer = memory_pools.entity_pool.get();
            drop(buffer);
            let _reused = memory_pools.entity_pool.get();
        });

        // Benchmark allocation throughput
        self.runner.benchmark_throughput("memory_allocation_throughput", 1000, || {
            let _buffer = memory_pools.packet_buffer_pool.get();
        });
    }

    /// Save benchmark results
    pub fn save_results(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.runner.save_results(path)
    }
}

// Test plugin and components for benchmarking
struct TestPlugin;

impl mirai_core::Plugin for TestPlugin {
    fn name(&self) -> &'static str {
        "benchmark_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestComponent>();
        app.world_mut().register_component::<PositionComponent>();
        app.add_system(test_system);
    }
}

#[derive(Debug, Clone)]
struct TestComponent {
    id: usize,
    data: String,
}

impl mirai_core::Component for TestComponent {}

#[derive(Debug, Clone)]
struct PositionComponent {
    x: f32,
    y: f32,
    z: f32,
}

impl mirai_core::Component for PositionComponent {}

fn test_system(world: &mut mirai_core::World) {
    for mut component in world.query::<&mut TestComponent>() {
        component.data = format!("processed_{}", component.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_comparison_benchmarks() {
        let mut comparison = BaselineComparison::new();
        let results = comparison.run_all_benchmarks();
        
        assert!(!results.is_empty());
        
        // Verify we have expected benchmark categories
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("server_startup")));
        assert!(names.iter().any(|n| n.contains("entity_spawn")));
        assert!(names.iter().any(|n| n.contains("component")));
        assert!(names.iter().any(|n| n.contains("system_execution")));
    }

    #[test]
    fn test_benchmark_result_serialization() {
        let result = BenchmarkResult {
            name: "test_benchmark".to_string(),
            value: 1.23,
            unit: "seconds".to_string(),
            lower_is_better: true,
            metadata: BenchmarkMetadata {
                timestamp: 1234567890,
                git_commit: Some("abc123".to_string()),
                test_config: "test".to_string(),
                system_info: SystemInfo {
                    os: "linux".to_string(),
                    arch: "x86_64".to_string(),
                    cpu_cores: 8,
                    memory_gb: 16.0,
                },
            },
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize");
        let deserialized: BenchmarkResult = serde_json::from_str(&json)
            .expect("Failed to deserialize");
        
        assert_eq!(result.name, deserialized.name);
        assert_eq!(result.value, deserialized.value);
    }
}