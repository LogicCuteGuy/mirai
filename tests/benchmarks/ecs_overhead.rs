//! ECS overhead benchmarks
//!
//! Validates that ECS integration doesn't significantly impact packet processing
//! and other core server operations

use super::*;
use mirai_core::{App, World, Instance};
use mirai_proto::{RakNetConnectionManager, RakNetManagerConfig, RawBedrockPacket, PacketDirection};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use bytes::Bytes;

/// ECS overhead benchmark suite
pub struct EcsOverheadBenchmarks {
    runner: BenchmarkRunner,
    rt: Runtime,
}

impl EcsOverheadBenchmarks {
    pub fn new() -> Self {
        Self {
            runner: BenchmarkRunner::new(BenchmarkConfig::default()),
            rt: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Run all ECS overhead benchmarks
    pub fn run_all_benchmarks(&mut self) -> Vec<BenchmarkResult> {
        println!("Running ECS overhead benchmarks...");

        self.benchmark_packet_processing_overhead();
        self.benchmark_entity_management_overhead();
        self.benchmark_system_execution_overhead();
        self.benchmark_component_access_overhead();
        self.benchmark_query_performance_overhead();

        self.runner.results().to_vec()
    }

    /// Benchmark packet processing with and without ECS
    fn benchmark_packet_processing_overhead(&mut self) {
        println!("Benchmarking packet processing overhead...");

        self.rt.block_on(async {
            // Setup network manager
            let addr = std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 
                0
            );
            let config = RakNetManagerConfig::default();
            let manager = RakNetConnectionManager::new(addr, config).await
                .expect("Failed to create RakNet manager");
            
            manager.start().await.expect("Failed to start manager");

            // Benchmark baseline packet processing (no ECS)
            let baseline_time = self.runner.benchmark("packet_processing_baseline", || {
                let packet = RawBedrockPacket {
                    id: 0x01,
                    data: Bytes::from("benchmark_packet_data"),
                    direction: PacketDirection::ClientToServer,
                };
                
                // Simulate basic packet processing
                let _processed_id = packet.id;
                let _data_len = packet.data.len();
                
                packet
            });

            // Setup ECS-enabled instance
            let mut app = App::new();
            app.add_plugin(EcsTestPlugin);
            let instance = app.build_instance().await
                .expect("Failed to build ECS instance");

            // Benchmark packet processing with ECS integration
            let ecs_time = self.runner.benchmark("packet_processing_with_ecs", || {
                let packet = RawBedrockPacket {
                    id: 0x01,
                    data: Bytes::from("benchmark_packet_data"),
                    direction: PacketDirection::ClientToServer,
                };
                
                // Simulate ECS-integrated packet processing
                let world = instance.world();
                let entity = world.spawn_entity();
                world.add_component(entity, PacketComponent {
                    packet_id: packet.id,
                    data_size: packet.data.len(),
                    timestamp: Instant::now(),
                });
                
                // Process through ECS
                world.run_systems();
                
                packet
            });

            // Calculate overhead
            let overhead_ratio = ecs_time.as_nanos() as f64 / baseline_time.as_nanos() as f64;
            self.runner.results.push(BenchmarkResult {
                name: "packet_processing_ecs_overhead".to_string(),
                value: overhead_ratio,
                unit: "ratio".to_string(),
                lower_is_better: true,
                metadata: self.runner.create_metadata("packet_processing_overhead"),
            });

            // Benchmark packet throughput with ECS
            let packet_count = 1000;
            let start = Instant::now();
            
            for i in 0..packet_count {
                let packet = RawBedrockPacket {
                    id: (i % 256) as u8,
                    data: Bytes::from(format!("packet_{}", i)),
                    direction: PacketDirection::ClientToServer,
                };
                
                let world = instance.world();
                let entity = world.spawn_entity();
                world.add_component(entity, PacketComponent {
                    packet_id: packet.id,
                    data_size: packet.data.len(),
                    timestamp: Instant::now(),
                });
            }
            
            // Process all packets through ECS
            instance.world().run_systems();
            
            let throughput_time = start.elapsed();
            let packets_per_second = packet_count as f64 / throughput_time.as_secs_f64();
            
            self.runner.results.push(BenchmarkResult {
                name: "ecs_packet_throughput".to_string(),
                value: packets_per_second,
                unit: "packets/sec".to_string(),
                lower_is_better: false,
                metadata: self.runner.create_metadata("ecs_packet_throughput"),
            });

            manager.stop().await.expect("Failed to stop manager");
        });
    }

    /// Benchmark entity management overhead
    fn benchmark_entity_management_overhead(&mut self) {
        println!("Benchmarking entity management overhead...");

        // Baseline entity management (simple Vec storage)
        let baseline_time = self.runner.benchmark("entity_management_baseline", || {
            let mut entities = Vec::new();
            for i in 0..1000 {
                entities.push(SimpleEntity {
                    id: i,
                    position: (i as f32, 0.0, 0.0),
                    data: format!("entity_{}", i),
                });
            }
            
            // Simulate processing
            for entity in &mut entities {
                entity.position.0 += 1.0;
            }
            
            entities
        });

        // ECS entity management
        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(EcsTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let ecs_time = self.runner.benchmark("entity_management_ecs", || {
            let world = instance.world();
            
            // Create entities
            for i in 0..1000 {
                let entity = world.spawn_entity();
                world.add_component(entity, PositionComponent {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                });
                world.add_component(entity, DataComponent {
                    id: i,
                    data: format!("entity_{}", i),
                });
            }
            
            // Process through systems
            world.run_systems();
        });

        // Calculate overhead
        let overhead_ratio = ecs_time.as_nanos() as f64 / baseline_time.as_nanos() as f64;
        self.runner.results.push(BenchmarkResult {
            name: "entity_management_ecs_overhead".to_string(),
            value: overhead_ratio,
            unit: "ratio".to_string(),
            lower_is_better: true,
            metadata: self.runner.create_metadata("entity_management_overhead"),
        });
    }

    /// Benchmark system execution overhead
    fn benchmark_system_execution_overhead(&mut self) {
        println!("Benchmarking system execution overhead...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(EcsTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Create test entities
        for i in 0..5000 {
            let entity = world.spawn_entity();
            world.add_component(entity, PositionComponent {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            });
            world.add_component(entity, VelocityComponent {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            });
        }

        // Benchmark single system execution
        self.runner.benchmark("single_system_execution", || {
            movement_system(world);
        });

        // Benchmark multiple system execution
        self.runner.benchmark("multiple_system_execution", || {
            movement_system(world);
            data_processing_system(world);
            cleanup_system(world);
        });

        // Benchmark system scheduling overhead
        self.runner.benchmark("system_scheduling_overhead", || {
            world.run_systems();
        });

        // Benchmark parallel system execution
        self.runner.benchmark("parallel_system_execution", || {
            // Simulate parallel system execution
            let start = Instant::now();
            
            // Run systems that can execute in parallel
            movement_system(world);
            data_processing_system(world);
            
            start.elapsed()
        });
    }

    /// Benchmark component access overhead
    fn benchmark_component_access_overhead(&mut self) {
        println!("Benchmarking component access overhead...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(EcsTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Create entities with components
        let mut entities = Vec::new();
        for i in 0..1000 {
            let entity = world.spawn_entity();
            world.add_component(entity, PositionComponent {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            });
            world.add_component(entity, DataComponent {
                id: i,
                data: format!("entity_{}", i),
            });
            entities.push(entity);
        }

        // Benchmark component access by entity ID
        self.runner.benchmark("component_access_by_id", || {
            for entity in &entities {
                let _pos = world.get_component::<PositionComponent>(*entity);
                let _data = world.get_component::<DataComponent>(*entity);
            }
        });

        // Benchmark component access through queries
        self.runner.benchmark("component_access_by_query", || {
            for (_pos, _data) in world.query::<(&PositionComponent, &DataComponent)>() {
                // Access component data
                let _x = _pos.x;
                let _id = _data.id;
            }
        });

        // Benchmark mutable component access
        self.runner.benchmark("mutable_component_access", || {
            for (mut pos, _data) in world.query::<(&mut PositionComponent, &DataComponent)>() {
                pos.x += 1.0;
            }
        });

        // Benchmark component addition/removal overhead
        self.runner.benchmark("component_add_remove_overhead", || {
            let entity = world.spawn_entity();
            
            // Add components
            world.add_component(entity, PositionComponent { x: 0.0, y: 0.0, z: 0.0 });
            world.add_component(entity, VelocityComponent { x: 1.0, y: 0.0, z: 0.0 });
            
            // Remove components
            world.remove_component::<VelocityComponent>(entity);
            world.remove_component::<PositionComponent>(entity);
        });
    }

    /// Benchmark query performance overhead
    fn benchmark_query_performance_overhead(&mut self) {
        println!("Benchmarking query performance overhead...");

        let instance = self.rt.block_on(async {
            let mut app = App::new();
            app.add_plugin(EcsTestPlugin);
            app.build_instance().await.expect("Failed to build instance")
        });

        let world = instance.world();

        // Create entities with various component combinations
        for i in 0..10000 {
            let entity = world.spawn_entity();
            world.add_component(entity, PositionComponent {
                x: i as f32,
                y: 0.0,
                z: 0.0,
            });
            
            if i % 2 == 0 {
                world.add_component(entity, VelocityComponent {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                });
            }
            
            if i % 3 == 0 {
                world.add_component(entity, DataComponent {
                    id: i,
                    data: format!("entity_{}", i),
                });
            }
        }

        // Benchmark simple queries
        self.runner.benchmark("simple_query_10k_entities", || {
            let mut count = 0;
            for _pos in world.query::<&PositionComponent>() {
                count += 1;
            }
            count
        });

        // Benchmark complex queries
        self.runner.benchmark("complex_query_10k_entities", || {
            let mut count = 0;
            for (_pos, _vel, _data) in world.query::<(&PositionComponent, &VelocityComponent, &DataComponent)>() {
                count += 1;
            }
            count
        });

        // Benchmark filtered queries
        self.runner.benchmark("filtered_query_10k_entities", || {
            let mut count = 0;
            for pos in world.query::<&PositionComponent>() {
                if pos.x > 5000.0 {
                    count += 1;
                }
            }
            count
        });

        // Benchmark query iteration overhead
        self.runner.benchmark("query_iteration_overhead", || {
            let start = Instant::now();
            
            // Multiple query iterations
            for _ in 0..10 {
                for _pos in world.query::<&PositionComponent>() {
                    // Minimal processing
                }
            }
            
            start.elapsed()
        });
    }

    /// Save benchmark results
    pub fn save_results(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.runner.save_results(path)
    }
}

// Test components and systems for ECS overhead benchmarking

struct EcsTestPlugin;

impl mirai_core::Plugin for EcsTestPlugin {
    fn name(&self) -> &'static str {
        "ecs_overhead_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<PacketComponent>();
        app.world_mut().register_component::<PositionComponent>();
        app.world_mut().register_component::<VelocityComponent>();
        app.world_mut().register_component::<DataComponent>();
        
        app.add_system(packet_processing_system);
        app.add_system(movement_system);
        app.add_system(data_processing_system);
        app.add_system(cleanup_system);
    }
}

#[derive(Debug, Clone)]
struct PacketComponent {
    packet_id: u8,
    data_size: usize,
    timestamp: Instant,
}

impl mirai_core::Component for PacketComponent {}

#[derive(Debug, Clone)]
struct PositionComponent {
    x: f32,
    y: f32,
    z: f32,
}

impl mirai_core::Component for PositionComponent {}

#[derive(Debug, Clone)]
struct VelocityComponent {
    x: f32,
    y: f32,
    z: f32,
}

impl mirai_core::Component for VelocityComponent {}

#[derive(Debug, Clone)]
struct DataComponent {
    id: usize,
    data: String,
}

impl mirai_core::Component for DataComponent {}

// Simple entity for baseline comparison
#[derive(Debug, Clone)]
struct SimpleEntity {
    id: usize,
    position: (f32, f32, f32),
    data: String,
}

// ECS systems for benchmarking
fn packet_processing_system(world: &mut mirai_core::World) {
    for packet in world.query::<&PacketComponent>() {
        // Simulate packet processing
        let _processing_time = packet.timestamp.elapsed();
        let _processed = packet.packet_id.wrapping_add(1);
    }
}

fn movement_system(world: &mut mirai_core::World) {
    for (mut pos, vel) in world.query::<(&mut PositionComponent, &VelocityComponent)>() {
        pos.x += vel.x;
        pos.y += vel.y;
        pos.z += vel.z;
    }
}

fn data_processing_system(world: &mut mirai_core::World) {
    for mut data in world.query::<&mut DataComponent>() {
        data.data = format!("processed_{}", data.id);
    }
}

fn cleanup_system(world: &mut mirai_core::World) {
    // Simulate cleanup operations
    let mut to_remove = Vec::new();
    
    for (entity, pos) in world.query_with_entity::<&PositionComponent>() {
        if pos.x > 10000.0 {
            to_remove.push(entity);
        }
    }
    
    for entity in to_remove {
        world.despawn_entity(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecs_overhead_benchmarks() {
        let mut benchmarks = EcsOverheadBenchmarks::new();
        let results = benchmarks.run_all_benchmarks();
        
        assert!(!results.is_empty());
        
        // Verify we have ECS-specific benchmarks
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.iter().any(|n| n.contains("ecs_overhead")));
        assert!(names.iter().any(|n| n.contains("packet_processing")));
        assert!(names.iter().any(|n| n.contains("entity_management")));
    }

    #[test]
    fn test_packet_processing_overhead_acceptable() {
        let mut benchmarks = EcsOverheadBenchmarks::new();
        benchmarks.benchmark_packet_processing_overhead();
        
        let results = benchmarks.runner.results();
        let overhead_result = results.iter()
            .find(|r| r.name == "packet_processing_ecs_overhead")
            .expect("Should have overhead measurement");
        
        // ECS overhead should be less than 50% (ratio < 1.5)
        assert!(overhead_result.value < 1.5, 
                "ECS overhead too high: {}x", overhead_result.value);
    }

    #[test]
    fn test_entity_management_performance() {
        let mut benchmarks = EcsOverheadBenchmarks::new();
        benchmarks.benchmark_entity_management_overhead();
        
        let results = benchmarks.runner.results();
        let overhead_result = results.iter()
            .find(|r| r.name == "entity_management_ecs_overhead")
            .expect("Should have entity management overhead");
        
        // Entity management overhead should be reasonable
        assert!(overhead_result.value < 3.0, 
                "Entity management overhead too high: {}x", overhead_result.value);
    }
}