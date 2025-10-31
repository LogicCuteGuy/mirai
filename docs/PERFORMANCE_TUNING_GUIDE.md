# Performance Tuning Guide: Optimizing Unified Mirai Server

This guide provides comprehensive instructions for optimizing the performance of your unified Mirai server using `UnifiedConfig` options and advanced tuning techniques.

## Table of Contents

1. [Performance Overview](#performance-overview)
2. [Configuration Optimization](#configuration-optimization)
3. [System-Level Tuning](#system-level-tuning)
4. [ECS Performance Optimization](#ecs-performance-optimization)
5. [Plugin Performance](#plugin-performance)
6. [Network Optimization](#network-optimization)
7. [Memory Management](#memory-management)
8. [Monitoring and Profiling](#monitoring-and-profiling)
9. [Troubleshooting Performance Issues](#troubleshooting-performance-issues)
10. [Advanced Optimization Techniques](#advanced-optimization-techniques)

## Performance Overview

The unified Mirai server provides several performance advantages over standard implementations:

- **ECS Architecture**: Efficient entity management with data-oriented design
- **Multi-threaded Processing**: Parallel system execution
- **Advanced Memory Management**: Object pooling and garbage collection optimization
- **Intelligent Caching**: Chunk and entity caching systems
- **Performance Monitoring**: Real-time metrics and alerting

### Performance Metrics

Key metrics to monitor:
- **TPS (Ticks Per Second)**: Target 20 TPS for smooth gameplay
- **Memory Usage**: RAM consumption and garbage collection frequency
- **CPU Usage**: Per-core utilization and thread efficiency
- **Network Latency**: Packet processing and transmission delays
- **Disk I/O**: World loading and saving performance

## Configuration Optimization

### Basic Performance Configuration

```toml
# unified_config.toml - Performance Optimized

[server]
server_name = "High Performance Mirai Server"
max_players = 100
max_connections = 120
view_distance = 10          # Reduce for better performance
simulation_distance = 6     # Lower than view distance
max_render_distance = 12    # Client-side limit
difficulty = "Hard"
gamemode = "Survival"
pvp = true
online_mode = true

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 120
timeout_seconds = 30
encryption_enabled = true

[network.compression]
algorithm = "flate"         # Fast compression
threshold = 256             # Compress packets > 256 bytes

[network.throttling]
enabled = true
scalar = 0.8               # Throttle to 80% of max bandwidth
threshold = 1000           # Throttle after 1000 packets/sec

[world]
world_name = "optimized_world"
level_path = "worlds/optimized_world"
seed = 12345
generate_structures = true
level_type = "default"
spawn_protection = 16
max_world_size = 29999984

# Performance Features
[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = false       # Disable if not needed
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

# ECS Optimization
[ecs]
enabled = true
system_thread_count = 8     # Match CPU cores
component_capacity = 50000  # Pre-allocate component storage
entity_capacity = 20000     # Pre-allocate entity storage
parallel_systems = true     # Enable parallel system execution
batch_size = 100           # Process entities in batches

# Plugin System Optimization
[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false          # Disable in production
max_plugins = 50
plugin_thread_pool = 4

# Performance Monitoring
[monitoring]
enabled = true
metrics_interval = "5s"     # Frequent monitoring
performance_alerts = true
detailed_profiling = false  # Enable only for debugging
memory_tracking = true
cpu_tracking = true

# Advanced Performance Settings
[performance]
# Thread Pool Configuration
thread_pool_size = 8        # Match CPU cores
worker_threads = 4          # Background workers
io_threads = 2             # I/O operations

# Memory Management
memory_pool_size = 2000     # Object pool size
gc_interval = "30s"        # Garbage collection interval
memory_limit = "8GB"       # Maximum memory usage

# Caching Configuration
[caching]
enable_chunk_cache = true
chunk_cache_size = 2000     # Number of chunks to cache
chunk_cache_ttl = "300s"    # Cache time-to-live

enable_entity_cache = true
entity_cache_size = 10000   # Number of entities to cache
entity_cache_ttl = "60s"

enable_packet_cache = true
packet_cache_size = 1000    # Cached packet structures

# I/O Optimization
[io]
async_world_loading = true
world_save_interval = "300s" # Save every 5 minutes
batch_world_saves = true
compression_level = 6       # Balance between speed and size

# Network Optimization
[network.optimization]
tcp_nodelay = true
socket_buffer_size = 65536
send_buffer_size = 32768
receive_buffer_size = 32768
```

### Environment-Specific Configurations

#### High-Performance Server (16+ cores, 32+ GB RAM)

```toml
[ecs]
system_thread_count = 16
component_capacity = 100000
entity_capacity = 50000
batch_size = 200

[performance]
thread_pool_size = 16
worker_threads = 8
io_threads = 4
memory_pool_size = 5000
memory_limit = "24GB"

[caching]
chunk_cache_size = 5000
entity_cache_size = 25000
packet_cache_size = 2000

[monitoring]
metrics_interval = "1s"
detailed_profiling = true
```

#### Medium Server (4-8 cores, 8-16 GB RAM)

```toml
[ecs]
system_thread_count = 6
component_capacity = 30000
entity_capacity = 15000
batch_size = 100

[performance]
thread_pool_size = 6
worker_threads = 3
io_threads = 2
memory_pool_size = 1500
memory_limit = "12GB"

[caching]
chunk_cache_size = 1500
entity_cache_size = 8000
packet_cache_size = 800
```

#### Low-Resource Server (2-4 cores, 4-8 GB RAM)

```toml
[ecs]
system_thread_count = 3
component_capacity = 15000
entity_capacity = 8000
batch_size = 50

[performance]
thread_pool_size = 3
worker_threads = 2
io_threads = 1
memory_pool_size = 800
memory_limit = "6GB"

[caching]
chunk_cache_size = 800
entity_cache_size = 4000
packet_cache_size = 400

[monitoring]
metrics_interval = "10s"
detailed_profiling = false
```

## System-Level Tuning

### Operating System Optimization

#### Linux Optimization

```bash
#!/bin/bash
# optimize_linux.sh

echo "Optimizing Linux system for Mirai server..."

# Increase file descriptor limits
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# Optimize network settings
cat >> /etc/sysctl.conf << EOF
# Network optimization for Mirai
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_congestion_control = bbr

# Memory optimization
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5

# CPU optimization
kernel.sched_migration_cost_ns = 5000000
kernel.sched_autogroup_enabled = 0
EOF

# Apply settings
sysctl -p

# Set CPU governor to performance
echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

echo "Linux optimization completed"
```

#### Windows Optimization

```powershell
# optimize_windows.ps1

Write-Host "Optimizing Windows system for Mirai server..."

# Set high performance power plan
powercfg -setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c

# Optimize network settings
netsh int tcp set global autotuninglevel=normal
netsh int tcp set global chimney=enabled
netsh int tcp set global rss=enabled
netsh int tcp set global netdma=enabled

# Set process priority
$process = Get-Process -Name "mirai" -ErrorAction SilentlyContinue
if ($process) {
    $process.PriorityClass = "High"
}

Write-Host "Windows optimization completed"
```

### JVM-like Optimization for Rust

```bash
# Set environment variables for optimal Rust performance
export RUST_BACKTRACE=0                    # Disable backtraces in production
export RUST_LOG=warn                       # Reduce logging overhead
export RUSTFLAGS="-C target-cpu=native"    # Optimize for current CPU

# Memory allocator optimization
export MALLOC_ARENA_MAX=4                  # Limit memory arenas
export MALLOC_MMAP_THRESHOLD_=131072       # Use mmap for large allocations
```

## ECS Performance Optimization

### System Ordering and Dependencies

```rust
// Optimal system ordering for performance
impl Plugin for OptimizedPlugin {
    fn build(&self, app: &mut App) -> Result<()> {
        // Stage 1: Input systems (parallel where possible)
        app.add_system_to_stage(InputSystem::new(), "input");
        app.add_system_to_stage(NetworkInputSystem::new(), "input");
        
        // Stage 2: Logic systems (ordered by dependencies)
        app.add_system_to_stage(PhysicsSystem::new(), "physics");
        app.add_system_to_stage(MovementSystem::new(), "movement");
        app.add_system_to_stage(CollisionSystem::new(), "collision");
        
        // Stage 3: Game logic (parallel where possible)
        app.add_system_to_stage(GameLogicSystem::new(), "game_logic");
        app.add_system_to_stage(AISystem::new(), "game_logic");
        
        // Stage 4: Output systems
        app.add_system_to_stage(RenderSystem::new(), "output");
        app.add_system_to_stage(NetworkOutputSystem::new(), "output");
        
        Ok(())
    }
}
```

### Component Optimization

```rust
// Optimized component design
#[derive(Component)]
pub struct OptimizedComponent {
    // Use primitive types when possible
    pub position: [f32; 3],     // Instead of Vec3
    pub velocity: [f32; 3],     // Packed data
    
    // Use bit flags for boolean values
    pub flags: u32,             // Instead of multiple bools
    
    // Cache frequently accessed data
    pub cached_distance: f32,
    pub last_update: u64,       // Timestamp as u64
}

impl OptimizedComponent {
    // Inline frequently called methods
    #[inline]
    pub fn is_moving(&self) -> bool {
        self.flags & 0x01 != 0
    }
    
    #[inline]
    pub fn set_moving(&mut self, moving: bool) {
        if moving {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
    }
}
```

### Query Optimization

```rust
// Efficient query patterns
impl System for OptimizedSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        // Use specific queries instead of broad ones
        let moving_entities: Vec<EntityId> = world
            .query::<(&Position, &Velocity)>()
            .iter()
            .filter(|(_, (_, vel))| vel.length() > 0.01)
            .map(|(id, _)| id)
            .collect();
        
        // Process in batches for better cache locality
        for chunk in moving_entities.chunks(100) {
            self.process_entity_batch(world, chunk)?;
        }
        
        Ok(())
    }
}
```

## Plugin Performance

### Plugin Optimization Guidelines

```rust
// High-performance plugin template
pub struct PerformantPlugin {
    // Use object pools for frequently created objects
    entity_pool: ObjectPool<EntityData>,
    
    // Cache expensive computations
    computation_cache: LruCache<String, ComputationResult>,
    
    // Batch operations
    pending_operations: Vec<Operation>,
    
    // Performance metrics
    metrics: PluginMetrics,
}

impl System for PerformantSystem {
    fn run(&mut self, world: &mut World) -> Result<()> {
        let start_time = Instant::now();
        
        // Batch entity processing
        let entities = self.get_entities_to_process(world);
        
        // Process in parallel if safe
        if self.is_parallel_safe() {
            entities.par_chunks(50)
                .for_each(|chunk| {
                    self.process_chunk_parallel(chunk);
                });
        } else {
            for chunk in entities.chunks(50) {
                self.process_chunk_sequential(world, chunk)?;
            }
        }
        
        // Record performance metrics
        let duration = start_time.elapsed();
        self.metrics.record_processing_time(duration);
        
        Ok(())
    }
    
    fn is_parallel_safe(&self) -> bool {
        true // Only if system doesn't modify shared state
    }
}
```

### Plugin Configuration for Performance

```toml
[plugins.high_performance_plugin]
enabled = true
update_interval = "50ms"    # 20 TPS
batch_size = 100
parallel_processing = true
cache_size = 1000
memory_pool_size = 500

[plugins.high_performance_plugin.performance]
enable_profiling = false    # Disable in production
max_processing_time = "10ms"
alert_threshold = "15ms"
```

## Network Optimization

### Packet Processing Optimization

```toml
[network.optimization]
# Packet batching
enable_packet_batching = true
batch_size = 50
batch_timeout = "5ms"

# Compression optimization
compression_level = 6       # Balance speed vs size
compression_threshold = 256 # Only compress larger packets

# Buffer optimization
send_buffer_size = 65536
receive_buffer_size = 65536
socket_buffer_size = 131072

# Connection optimization
tcp_nodelay = true
keep_alive = true
keep_alive_interval = "30s"

# Rate limiting
enable_rate_limiting = true
packets_per_second = 1000
burst_size = 100
```

### Network Monitoring

```rust
// Network performance monitoring
pub struct NetworkMonitor {
    packet_counts: HashMap<PacketType, u64>,
    bandwidth_usage: RingBuffer<u64>,
    latency_samples: RingBuffer<Duration>,
}

impl NetworkMonitor {
    pub fn record_packet(&mut self, packet_type: PacketType, size: usize) {
        *self.packet_counts.entry(packet_type).or_insert(0) += 1;
        self.bandwidth_usage.push(size as u64);
    }
    
    pub fn get_performance_stats(&self) -> NetworkStats {
        NetworkStats {
            packets_per_second: self.calculate_pps(),
            bandwidth_mbps: self.calculate_bandwidth(),
            average_latency: self.calculate_average_latency(),
        }
    }
}
```

## Memory Management

### Memory Configuration

```toml
[memory]
# Heap management
initial_heap_size = "2GB"
max_heap_size = "8GB"
heap_growth_factor = 1.5

# Object pooling
enable_object_pooling = true
pool_sizes = { "Entity" = 10000, "Component" = 50000, "Packet" = 5000 }

# Garbage collection
gc_strategy = "generational"
gc_interval = "30s"
gc_threshold = 0.8          # Trigger GC at 80% memory usage

# Memory monitoring
enable_memory_tracking = true
memory_alert_threshold = 0.9 # Alert at 90% usage
```

### Memory Optimization Techniques

```rust
// Memory-efficient data structures
pub struct MemoryOptimizedWorld {
    // Use packed arrays for better cache locality
    positions: PackedArray<Position>,
    velocities: PackedArray<Velocity>,
    
    // Use object pools for frequent allocations
    entity_pool: ObjectPool<Entity>,
    component_pool: ObjectPool<Component>,
    
    // Use arena allocation for temporary objects
    temp_arena: Arena,
}

impl MemoryOptimizedWorld {
    pub fn process_entities(&mut self) {
        // Reset arena for each frame
        self.temp_arena.reset();
        
        // Use arena for temporary allocations
        let temp_data = self.temp_arena.alloc(TempData::new());
        
        // Process entities with good cache locality
        for (pos, vel) in self.positions.iter().zip(self.velocities.iter()) {
            self.process_entity_optimized(pos, vel, temp_data);
        }
    }
}
```

## Monitoring and Profiling

### Performance Monitoring Configuration

```toml
[monitoring]
enabled = true
metrics_interval = "5s"
performance_alerts = true
detailed_profiling = false

# Metrics to collect
[monitoring.metrics]
cpu_usage = true
memory_usage = true
network_stats = true
disk_io = true
ecs_performance = true
plugin_performance = true

# Alert thresholds
[monitoring.alerts]
cpu_threshold = 80.0        # Alert at 80% CPU usage
memory_threshold = 85.0     # Alert at 85% memory usage
tps_threshold = 18.0        # Alert if TPS drops below 18
latency_threshold = "100ms" # Alert if latency exceeds 100ms

# Performance logging
[monitoring.logging]
log_level = "info"
log_file = "performance.log"
rotate_logs = true
max_log_size = "100MB"
```

### Real-time Performance Dashboard

```bash
# Start performance dashboard
mirai dashboard --port 8080 --config unified_config.toml

# Access dashboard at http://localhost:8080
# Metrics available:
# - Real-time TPS
# - Memory usage graphs
# - CPU utilization
# - Network statistics
# - Plugin performance
# - ECS system timings
```

### Profiling Commands

```bash
# CPU profiling
mirai profile cpu --duration 60s --output cpu_profile.json

# Memory profiling
mirai profile memory --duration 30s --output memory_profile.json

# Network profiling
mirai profile network --duration 120s --output network_profile.json

# Plugin profiling
mirai profile plugins --plugin-name my_plugin --duration 60s
```

## Troubleshooting Performance Issues

### Common Performance Problems

#### 1. Low TPS (Ticks Per Second)

```bash
# Diagnose TPS issues
mirai analyze tps --log-file server.log --duration 300s

# Common causes and solutions:
# - High CPU usage: Reduce thread count, optimize plugins
# - Memory pressure: Increase heap size, enable GC tuning
# - I/O bottleneck: Use SSD, optimize world saving
# - Plugin overhead: Profile plugins, disable unnecessary ones
```

#### 2. High Memory Usage

```bash
# Memory analysis
mirai analyze memory --config unified_config.toml

# Solutions:
# - Reduce cache sizes
# - Enable object pooling
# - Tune garbage collection
# - Check for memory leaks in plugins
```

#### 3. Network Latency

```bash
# Network diagnostics
mirai test network --target-latency 50ms --duration 60s

# Optimizations:
# - Enable packet batching
# - Optimize compression settings
# - Tune buffer sizes
# - Check network infrastructure
```

### Performance Debugging

```rust
// Performance debugging utilities
pub struct PerformanceDebugger {
    system_timings: HashMap<String, Vec<Duration>>,
    memory_snapshots: Vec<MemorySnapshot>,
    frame_times: RingBuffer<Duration>,
}

impl PerformanceDebugger {
    pub fn profile_system<F, R>(&mut self, system_name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        
        self.system_timings
            .entry(system_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
        
        if duration > Duration::from_millis(10) {
            tracing::warn!("Slow system: {} took {:?}", system_name, duration);
        }
        
        result
    }
    
    pub fn generate_report(&self) -> PerformanceReport {
        PerformanceReport {
            average_frame_time: self.calculate_average_frame_time(),
            slowest_systems: self.find_slowest_systems(5),
            memory_trend: self.analyze_memory_trend(),
            recommendations: self.generate_recommendations(),
        }
    }
}
```

## Advanced Optimization Techniques

### Custom Memory Allocators

```rust
// Custom allocator for specific use cases
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

// Or use jemalloc for better performance
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
```

### SIMD Optimization

```rust
// Use SIMD for vector operations
use std::simd::*;

pub fn update_positions_simd(positions: &mut [f32], velocities: &[f32], dt: f32) {
    assert_eq!(positions.len(), velocities.len());
    
    let dt_vec = f32x4::splat(dt);
    
    for (pos_chunk, vel_chunk) in positions.chunks_exact_mut(4).zip(velocities.chunks_exact(4)) {
        let pos = f32x4::from_slice(pos_chunk);
        let vel = f32x4::from_slice(vel_chunk);
        let new_pos = pos + vel * dt_vec;
        new_pos.copy_to_slice(pos_chunk);
    }
}
```

### Lock-Free Data Structures

```rust
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicU64, Ordering};

// Lock-free event queue for high-performance event handling
pub struct LockFreeEventQueue<T> {
    queue: SegQueue<T>,
    counter: AtomicU64,
}

impl<T> LockFreeEventQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: SegQueue::new(),
            counter: AtomicU64::new(0),
        }
    }
    
    pub fn push(&self, event: T) {
        self.queue.push(event);
        self.counter.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn pop(&self) -> Option<T> {
        self.queue.pop()
    }
}
```

### Benchmark-Driven Optimization

```rust
// Benchmarking framework for optimization
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_entity_processing(c: &mut Criterion) {
    let mut world = create_test_world_with_entities(10000);
    
    c.bench_function("entity_processing", |b| {
        b.iter(|| {
            let mut system = MovementSystem::new();
            system.run(black_box(&mut world)).unwrap();
        })
    });
}

criterion_group!(benches, benchmark_entity_processing);
criterion_main!(benches);
```

### Configuration Templates

```bash
#!/bin/bash
# generate_optimized_config.sh

CPU_CORES=$(nproc)
TOTAL_RAM_GB=$(free -g | awk 'NR==2{print $2}')
EXPECTED_PLAYERS=${1:-50}

echo "Generating optimized configuration..."
echo "CPU Cores: $CPU_CORES"
echo "RAM: ${TOTAL_RAM_GB}GB"
echo "Expected Players: $EXPECTED_PLAYERS"

# Calculate optimal settings
THREAD_COUNT=$((CPU_CORES > 8 ? 8 : CPU_CORES))
MEMORY_LIMIT=$((TOTAL_RAM_GB * 80 / 100))  # Use 80% of available RAM
CACHE_SIZE=$((EXPECTED_PLAYERS * 20))      # 20 chunks per player

cat > optimized_config.toml << EOF
[server]
max_players = $EXPECTED_PLAYERS
view_distance = $((EXPECTED_PLAYERS > 100 ? 8 : 10))
simulation_distance = $((EXPECTED_PLAYERS > 100 ? 4 : 6))

[ecs]
system_thread_count = $THREAD_COUNT
component_capacity = $((EXPECTED_PLAYERS * 500))
entity_capacity = $((EXPECTED_PLAYERS * 200))

[performance]
thread_pool_size = $THREAD_COUNT
memory_limit = "${MEMORY_LIMIT}GB"

[caching]
chunk_cache_size = $CACHE_SIZE
entity_cache_size = $((EXPECTED_PLAYERS * 100))
EOF

echo "Optimized configuration generated: optimized_config.toml"
```

This performance tuning guide provides comprehensive optimization strategies for the unified Mirai server. Regular monitoring and iterative optimization based on your specific workload will yield the best results.