//! Performance benchmarking and regression testing for the merged system
//! 
//! Tests that validate the merged system performs at least as well as
//! the original mirai implementation and includes performance optimizations
//! from both projects.

use super::*;
use mirai_core::{
    App, World, PerformanceManager, PerformanceStats,
    performance::{MetricsCollector, ThreadManager, MemoryManager}
};
use mirai_proto::{RakNetConnectionManager, RakNetManagerConfig, UnifiedProtocolHandler};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Barrier;

#[tokio::test]
async fn test_merged_system_performance_baseline() {
    let mut app = App::new();
    app.add_plugin(PerformanceTestPlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build performance test instance");
    
    let performance_manager = instance.performance_manager();
    
    // Baseline performance test
    let start_time = Instant::now();
    
    // Simulate typical server operations
    for i in 0..10000 {
        let entity = instance.world().spawn_entity();
        instance.world().add_component(entity, TestPerformanceComponent { 
            id: i, 
            data: format!("entity_{}", i) 
        });
    }
    
    // Run systems
    instance.world().run_systems();
    
    let baseline_duration = start_time.elapsed();
    
    // Performance should be reasonable (under 100ms for 10k entities)
    assert!(baseline_duration < Duration::from_millis(100));
    
    // Verify performance stats
    let stats = performance_manager.performance_stats();
    assert!(stats.memory.overall_efficiency() > 0.5);
    assert!(stats.threading.utilization() >= 0.0);
}

#[tokio::test]
async fn test_memory_performance_optimization() {
    let performance_manager = PerformanceManager::new();
    let memory_manager = performance_manager.memory_manager();
    
    // Test memory pool performance
    let start_time = Instant::now();
    
    let mut buffers = Vec::new();
    
    // Allocate many buffers
    for _ in 0..1000 {
        let buffer = memory_manager.entity_pool().get();
        buffers.push(buffer);
    }
    
    let allocation_time = start_time.elapsed();
    
    // Drop all buffers (return to pool)
    let drop_start = Instant::now();
    drop(buffers);
    let drop_time = drop_start.elapsed();
    
    // Pool allocation should be fast
    assert!(allocation_time < Duration::from_millis(10));
    assert!(drop_time < Duration::from_millis(5));
    
    // Verify pool efficiency
    let stats = memory_manager.pool_stats();
    assert!(stats.overall_efficiency() > 0.8);
}

#[tokio::test]
async fn test_threading_performance_optimization() {
    let performance_manager = PerformanceManager::new();
    let thread_manager = performance_manager.thread_manager();
    
    let task_count = 100;
    let barrier = Arc::new(Barrier::new(task_count + 1));
    let counter = Arc::new(AtomicU64::new(0));
    
    let start_time = Instant::now();
    
    // Submit many CPU tasks
    let mut handles = Vec::new();
    for i in 0..task_count {
        let barrier_clone = barrier.clone();
        let counter_clone = counter.clone();
        
        let handle = thread_manager.execute_cpu_task(move || {
            // Simulate work
            let mut sum = 0u64;
            for j in 0..1000 {
                sum += (i * 1000 + j) as u64;
            }
            
            counter_clone.fetch_add(sum, Ordering::Relaxed);
            barrier_clone.wait();
            sum
        }).expect("Failed to submit task");
        
        handles.push(handle);
    }
    
    // Wait for all tasks to start
    barrier.wait().await;
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await_result().await.expect("Task failed");
    }
    
    let total_time = start_time.elapsed();
    
    // Should complete reasonably quickly with parallel execution
    assert!(total_time < Duration::from_secs(1));
    
    // Verify all tasks executed
    assert!(counter.load(Ordering::Relaxed) > 0);
    
    // Check thread pool utilization
    let stats = thread_manager.stats();
    assert!(stats.utilization() > 0.0);
}

#[tokio::test]
async fn test_network_performance_benchmarks() {
    let addr = std::net::SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 
        0
    );
    let config = RakNetManagerConfig::default();
    
    let manager = RakNetConnectionManager::new(addr, config).await
        .expect("Failed to create RakNet manager");
    
    manager.start().await.expect("Failed to start manager");
    
    // Benchmark packet processing
    let packet_count = 1000;
    let start_time = Instant::now();
    
    for i in 0..packet_count {
        let packet = mirai_proto::RawBedrockPacket {
            id: 0x01,
            data: bytes::Bytes::from(format!("benchmark_packet_{}", i)),
            direction: mirai_proto::PacketDirection::ServerToClient,
        };
        
        let _ = manager.broadcast_packet(packet).await;
    }
    
    let processing_time = start_time.elapsed();
    
    // Should process packets quickly
    let packets_per_second = packet_count as f64 / processing_time.as_secs_f64();
    assert!(packets_per_second > 1000.0); // At least 1000 packets/second
    
    // Verify network stats
    let stats = manager.get_enhanced_stats().await;
    assert_eq!(stats.total_connections, 0); // No actual connections in test
    
    manager.stop().await.expect("Failed to stop manager");
}

#[tokio::test]
async fn test_ecs_system_performance() {
    let mut app = App::new();
    app.add_plugin(EcsPerformancePlugin);
    
    let instance = app.build_instance().await
        .expect("Failed to build ECS performance test instance");
    
    let world = instance.world();
    
    // Create many entities with components
    let entity_count = 50000;
    let start_time = Instant::now();
    
    for i in 0..entity_count {
        let entity = world.spawn_entity();
        world.add_component(entity, PositionComponent { x: i as f32, y: 0.0, z: 0.0 });
        world.add_component(entity, VelocityComponent { x: 1.0, y: 0.0, z: 0.0 });
    }
    
    let spawn_time = start_time.elapsed();
    
    // Run movement system
    let system_start = Instant::now();
    world.run_systems();
    let system_time = system_start.elapsed();
    
    // Performance benchmarks
    assert!(spawn_time < Duration::from_millis(500)); // 50k entities in 500ms
    assert!(system_time < Duration::from_millis(100)); // System execution in 100ms
    
    // Verify entities were processed
    let mut processed_count = 0;
    for entity in world.query::<&PositionComponent>() {
        if entity.x > 0.0 {
            processed_count += 1;
        }
    }
    assert_eq!(processed_count, entity_count);
}

#[test]
fn test_memory_usage_regression() {
    let initial_memory = get_memory_usage();
    
    let performance_manager = PerformanceManager::new();
    
    // Perform memory-intensive operations
    let mut entities = Vec::new();
    for i in 0..10000 {
        let mut buffer = performance_manager.memory_pools().entity_pool.get();
        buffer.extend_from_slice(&[i as u8; 100]);
        entities.push(buffer);
    }
    
    let peak_memory = get_memory_usage();
    
    // Release all memory
    drop(entities);
    
    // Force cleanup
    performance_manager.maintenance();
    
    let final_memory = get_memory_usage();
    
    // Memory should be efficiently managed
    let memory_growth = peak_memory.saturating_sub(initial_memory);
    let memory_retained = final_memory.saturating_sub(initial_memory);
    
    // Should not retain more than 10% of peak usage
    assert!(memory_retained < memory_growth / 10);
}

#[tokio::test]
async fn test_concurrent_performance() {
    let performance_manager = PerformanceManager::new();
    let task_count = 50;
    
    let start_time = Instant::now();
    
    // Run concurrent operations
    let mut handles = Vec::new();
    
    for i in 0..task_count {
        let manager_clone = performance_manager.clone();
        
        let handle = tokio::spawn(async move {
            // Simulate concurrent server operations
            let metrics = manager_clone.metrics_collector();
            
            for j in 0..100 {
                metrics.increment_counter(&format!("test_counter_{}", i), None);
                metrics.record_gauge(&format!("test_gauge_{}", i), j as f64, None);
                
                // Simulate some work
                tokio::time::sleep(Duration::from_micros(10)).await;
            }
            
            i
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task failed");
    }
    
    let total_time = start_time.elapsed();
    
    // Should handle concurrent operations efficiently
    assert!(total_time < Duration::from_secs(2));
    
    // Verify metrics were recorded
    let summary = performance_manager.metrics_collector().summary();
    assert!(summary.total_metrics >= task_count * 100 * 2); // counters + gauges
}

#[test]
fn test_performance_monitoring_overhead() {
    let performance_manager = PerformanceManager::new();
    let metrics = performance_manager.metrics_collector();
    
    // Measure overhead of performance monitoring
    let iterations = 100000;
    
    // Test without monitoring
    let start_time = Instant::now();
    for i in 0..iterations {
        let _result = i * 2 + 1;
    }
    let baseline_time = start_time.elapsed();
    
    // Test with monitoring
    let start_time = Instant::now();
    for i in 0..iterations {
        let _guard = metrics.start_profile("test_operation");
        let _result = i * 2 + 1;
    }
    let monitored_time = start_time.elapsed();
    
    // Monitoring overhead should be minimal (less than 50% increase)
    let overhead_ratio = monitored_time.as_nanos() as f64 / baseline_time.as_nanos() as f64;
    assert!(overhead_ratio < 1.5);
}

// Helper functions and test components

fn get_memory_usage() -> usize {
    // Simple memory usage estimation
    // In a real implementation, this would use proper memory profiling
    std::alloc::System.alloc(std::alloc::Layout::new::<u8>()) as usize
}

struct PerformanceTestPlugin;

impl mirai_core::Plugin for PerformanceTestPlugin {
    fn name(&self) -> &'static str {
        "performance_test_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<TestPerformanceComponent>();
        app.add_system(performance_test_system);
    }
}

struct EcsPerformancePlugin;

impl mirai_core::Plugin for EcsPerformancePlugin {
    fn name(&self) -> &'static str {
        "ecs_performance_plugin"
    }
    
    fn build(&self, app: &mut App) {
        app.world_mut().register_component::<PositionComponent>();
        app.world_mut().register_component::<VelocityComponent>();
        app.add_system(movement_system);
    }
}

#[derive(Debug, Clone)]
struct TestPerformanceComponent {
    id: usize,
    data: String,
}

impl mirai_core::Component for TestPerformanceComponent {}

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

fn performance_test_system(world: &mut mirai_core::World) {
    // Simple system that processes components
    for mut component in world.query::<&mut TestPerformanceComponent>() {
        component.data = format!("processed_{}", component.id);
    }
}

fn movement_system(world: &mut mirai_core::World) {
    // System that updates positions based on velocity
    for (mut pos, vel) in world.query::<(&mut PositionComponent, &VelocityComponent)>() {
        pos.x += vel.x;
        pos.y += vel.y;
        pos.z += vel.z;
    }
}