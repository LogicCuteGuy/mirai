# Example Configurations for Different Server Setups

This document provides ready-to-use configuration examples for various server deployment scenarios using the unified Mirai server.

## Table of Contents

1. [Small Private Server](#small-private-server)
2. [Medium Community Server](#medium-community-server)
3. [Large Public Server](#large-public-server)
4. [High-Performance Gaming Server](#high-performance-gaming-server)
5. [Development and Testing Server](#development-and-testing-server)
6. [Minimal Resource Server](#minimal-resource-server)
7. [Creative Building Server](#creative-building-server)
8. [Survival Hardcore Server](#survival-hardcore-server)
9. [Plugin Development Server](#plugin-development-server)
10. [Load Testing Server](#load-testing-server)

## Small Private Server

**Use Case:** 5-10 friends playing together
**Resources:** 2-4 CPU cores, 4-8 GB RAM
**Features:** Basic gameplay with essential plugins

```toml
# small_private_server.toml

[server]
server_name = "Friends Only Server"
motd = "Welcome to our private world!"
max_players = 10
max_connections = 15
view_distance = 10
simulation_distance = 6
max_render_distance = 12
difficulty = "Normal"
gamemode = "Survival"
hardcore = false
pvp = true
online_mode = true
whitelist = true
enforce_whitelist = true

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 15
timeout_seconds = 30
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 256

[network.throttling]
enabled = false

[world]
world_name = "friends_world"
level_path = "worlds/friends_world"
seed = 123456789
generate_structures = true
generator_settings = "{}"
level_type = "default"
spawn_protection = 16
max_world_size = 10000000

[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = false
command_system = true
performance_monitoring = false
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 2
component_capacity = 5000
entity_capacity = 2000
parallel_systems = false
batch_size = 50

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false
max_plugins = 10

[monitoring]
enabled = false
metrics_interval = "60s"
performance_alerts = false
detailed_profiling = false

[performance]
thread_pool_size = 2
worker_threads = 1
io_threads = 1
memory_pool_size = 500
memory_limit = "3GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 300
chunk_cache_ttl = "300s"
enable_entity_cache = true
entity_cache_size = 1000
entity_cache_ttl = "60s"

[io]
async_world_loading = true
world_save_interval = "300s"
batch_world_saves = true
compression_level = 6

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = false
```

## Medium Community Server

**Use Case:** 20-50 active players
**Resources:** 4-8 CPU cores, 8-16 GB RAM
**Features:** Full feature set with community plugins

```toml
# medium_community_server.toml

[server]
server_name = "Community Minecraft Server"
motd = "Join our growing community!"
max_players = 50
max_connections = 75
view_distance = 12
simulation_distance = 8
max_render_distance = 16
difficulty = "Hard"
gamemode = "Survival"
hardcore = false
pvp = true
online_mode = true
whitelist = false
enforce_whitelist = false

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 75
timeout_seconds = 30
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 256

[network.throttling]
enabled = true
scalar = 0.9
threshold = 500

[world]
world_name = "community_world"
level_path = "worlds/community_world"
seed = 987654321
generate_structures = true
generator_settings = "{\"biome_size\":4,\"river_size\":4}"
level_type = "default"
spawn_protection = 20
max_world_size = 29999984

[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = true
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 6
component_capacity = 25000
entity_capacity = 10000
parallel_systems = true
batch_size = 100

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false
max_plugins = 25

# Plugin-specific configurations
[plugins.economy]
enabled = true
starting_balance = 1000.0
currency_name = "coins"

[plugins.protection]
enabled = true
max_claims_per_player = 5
claim_size_limit = 100

[plugins.teleport]
enabled = true
cooldown_seconds = 30
cost_per_teleport = 10.0

[monitoring]
enabled = true
metrics_interval = "30s"
performance_alerts = true
detailed_profiling = false
memory_tracking = true
cpu_tracking = true

[performance]
thread_pool_size = 6
worker_threads = 3
io_threads = 2
memory_pool_size = 1500
memory_limit = "12GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 1000
chunk_cache_ttl = "300s"
enable_entity_cache = true
entity_cache_size = 5000
entity_cache_ttl = "60s"
enable_packet_cache = true
packet_cache_size = 500

[io]
async_world_loading = true
world_save_interval = "300s"
batch_world_saves = true
compression_level = 6

[security]
audit_logging = true
rate_limiting = true
ddos_protection = true
max_login_attempts = 5
login_timeout = "300s"

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = false
```

## Large Public Server

**Use Case:** 100+ concurrent players
**Resources:** 8+ CPU cores, 16+ GB RAM
**Features:** Full optimization for high player count

```toml
# large_public_server.toml

[server]
server_name = "Mega Public Server"
motd = "Welcome to the biggest server!"
max_players = 200
max_connections = 250
view_distance = 10
simulation_distance = 6
max_render_distance = 14
difficulty = "Hard"
gamemode = "Survival"
hardcore = false
pvp = true
online_mode = true
whitelist = false
enforce_whitelist = false

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 250
timeout_seconds = 20
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 128

[network.throttling]
enabled = true
scalar = 0.8
threshold = 1000

[network.optimization]
tcp_nodelay = true
socket_buffer_size = 131072
send_buffer_size = 65536
receive_buffer_size = 65536
enable_packet_batching = true
batch_size = 50
batch_timeout = "5ms"

[world]
world_name = "public_world"
level_path = "worlds/public_world"
seed = 192837465
generate_structures = true
generator_settings = "{\"biome_size\":4,\"river_size\":4,\"ore_size\":8}"
level_type = "default"
spawn_protection = 50
max_world_size = 29999984

[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = false
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 12
component_capacity = 100000
entity_capacity = 50000
parallel_systems = true
batch_size = 200

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false
max_plugins = 50
plugin_thread_pool = 6

# Extensive plugin configuration
[plugins.economy]
enabled = true
starting_balance = 500.0
currency_name = "credits"
bank_interest_rate = 0.01

[plugins.protection]
enabled = true
max_claims_per_player = 10
claim_size_limit = 200
protection_blocks = ["diamond_block", "emerald_block"]

[plugins.teleport]
enabled = true
cooldown_seconds = 60
cost_per_teleport = 50.0
max_homes_per_player = 5

[plugins.auction_house]
enabled = true
max_listings_per_player = 10
listing_fee_percentage = 0.05

[plugins.chat_management]
enabled = true
anti_spam = true
chat_cooldown = 2
profanity_filter = true

[monitoring]
enabled = true
metrics_interval = "10s"
performance_alerts = true
detailed_profiling = true
memory_tracking = true
cpu_tracking = true

[monitoring.alerts]
cpu_threshold = 85.0
memory_threshold = 90.0
tps_threshold = 18.0
latency_threshold = "150ms"

[performance]
thread_pool_size = 12
worker_threads = 6
io_threads = 4
memory_pool_size = 5000
memory_limit = "24GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 3000
chunk_cache_ttl = "600s"
enable_entity_cache = true
entity_cache_size = 15000
entity_cache_ttl = "120s"
enable_packet_cache = true
packet_cache_size = 2000

[io]
async_world_loading = true
world_save_interval = "180s"
batch_world_saves = true
compression_level = 4

[security]
audit_logging = true
rate_limiting = true
ddos_protection = true
max_login_attempts = 3
login_timeout = "600s"
ip_whitelist = false
geo_blocking = false

[mirai]
enable_legacy_api = false
preserve_existing_behavior = false
migration_mode = false
```

## High-Performance Gaming Server

**Use Case:** Competitive gaming with minimal latency
**Resources:** High-end hardware, optimized for performance
**Features:** Maximum performance optimization

```toml
# high_performance_gaming.toml

[server]
server_name = "Ultra Performance Server"
motd = "Competitive Gaming - Low Latency"
max_players = 100
max_connections = 120
view_distance = 8
simulation_distance = 4
max_render_distance = 10
difficulty = "Hard"
gamemode = "Survival"
hardcore = true
pvp = true
online_mode = true
whitelist = true
enforce_whitelist = true

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 120
timeout_seconds = 15
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 512

[network.throttling]
enabled = false

[network.optimization]
tcp_nodelay = true
socket_buffer_size = 262144
send_buffer_size = 131072
receive_buffer_size = 131072
enable_packet_batching = true
batch_size = 20
batch_timeout = "1ms"

[world]
world_name = "competitive_world"
level_path = "worlds/competitive_world"
seed = 555666777
generate_structures = false
generator_settings = "{\"biome_size\":2,\"river_size\":2}"
level_type = "flat"
spawn_protection = 0
max_world_size = 5000000

[features]
vanilla_mobs = false
redstone = true
world_generation = false
creative_mode = false
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = false

[ecs]
enabled = true
system_thread_count = 16
component_capacity = 50000
entity_capacity = 20000
parallel_systems = true
batch_size = 500

[plugins]
enabled = false

[monitoring]
enabled = true
metrics_interval = "1s"
performance_alerts = true
detailed_profiling = true
memory_tracking = true
cpu_tracking = true

[performance]
thread_pool_size = 16
worker_threads = 8
io_threads = 4
memory_pool_size = 10000
memory_limit = "32GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 5000
chunk_cache_ttl = "1200s"
enable_entity_cache = true
entity_cache_size = 25000
entity_cache_ttl = "300s"
enable_packet_cache = true
packet_cache_size = 5000

[io]
async_world_loading = true
world_save_interval = "60s"
batch_world_saves = true
compression_level = 1

[security]
audit_logging = false
rate_limiting = false
ddos_protection = true

[mirai]
enable_legacy_api = false
preserve_existing_behavior = false
migration_mode = false
```

## Development and Testing Server

**Use Case:** Plugin development and testing
**Resources:** Variable, optimized for debugging
**Features:** Development tools and debugging enabled

```toml
# development_testing.toml

[server]
server_name = "Development Server"
motd = "Development Environment - Unstable"
max_players = 5
max_connections = 10
view_distance = 6
simulation_distance = 4
max_render_distance = 8
difficulty = "Peaceful"
gamemode = "Creative"
hardcore = false
pvp = false
online_mode = false
whitelist = false
enforce_whitelist = false

[network]
port = 19133
ipv4_addr = "127.0.0.1"
max_clients = 10
timeout_seconds = 60
encryption_enabled = false

[network.compression]
algorithm = "flate"
threshold = 1024

[world]
world_name = "dev_world"
level_path = "worlds/dev_world"
seed = 12345
generate_structures = true
generator_settings = "{}"
level_type = "default"
spawn_protection = 0
max_world_size = 1000000

[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = true
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 2
component_capacity = 5000
entity_capacity = 1000
parallel_systems = false
batch_size = 10

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = true
max_plugins = 100
plugin_thread_pool = 1

# Development plugin configurations
[plugins.debug_tools]
enabled = true
verbose_logging = true
performance_tracking = true

[plugins.test_framework]
enabled = true
auto_run_tests = false
test_timeout = "30s"

[monitoring]
enabled = true
metrics_interval = "5s"
performance_alerts = false
detailed_profiling = true
memory_tracking = true
cpu_tracking = true

[monitoring.logging]
log_level = "debug"
log_file = "dev_server.log"
rotate_logs = false

[performance]
thread_pool_size = 2
worker_threads = 1
io_threads = 1
memory_pool_size = 100
memory_limit = "2GB"

[caching]
enable_chunk_cache = false
enable_entity_cache = false
enable_packet_cache = false

[io]
async_world_loading = false
world_save_interval = "60s"
batch_world_saves = false
compression_level = 9

[security]
audit_logging = false
rate_limiting = false
ddos_protection = false

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = true

# Development-specific settings
[development]
enable_debug_commands = true
allow_unsafe_operations = true
skip_validation = false
verbose_errors = true
```

## Minimal Resource Server

**Use Case:** Running on limited hardware (VPS, Raspberry Pi)
**Resources:** 1-2 CPU cores, 1-4 GB RAM
**Features:** Minimal feature set for resource conservation

```toml
# minimal_resource.toml

[server]
server_name = "Minimal Server"
motd = "Lightweight Minecraft Server"
max_players = 10
max_connections = 12
view_distance = 6
simulation_distance = 3
max_render_distance = 8
difficulty = "Easy"
gamemode = "Survival"
hardcore = false
pvp = false
online_mode = true
whitelist = true
enforce_whitelist = true

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 12
timeout_seconds = 45
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 128

[world]
world_name = "minimal_world"
level_path = "worlds/minimal_world"
seed = 54321
generate_structures = false
generator_settings = "{\"biome_size\":2}"
level_type = "default"
spawn_protection = 10
max_world_size = 5000000

[features]
vanilla_mobs = false
redstone = false
world_generation = true
creative_mode = false
command_system = false
performance_monitoring = false
ecs_system = false
plugin_system = false

[ecs]
enabled = false

[plugins]
enabled = false

[monitoring]
enabled = false

[performance]
thread_pool_size = 1
worker_threads = 1
io_threads = 1
memory_pool_size = 100
memory_limit = "1GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 100
chunk_cache_ttl = "180s"
enable_entity_cache = false
enable_packet_cache = false

[io]
async_world_loading = false
world_save_interval = "600s"
batch_world_saves = false
compression_level = 9

[security]
audit_logging = false
rate_limiting = false
ddos_protection = false

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = false
```

## Creative Building Server

**Use Case:** Creative building and architectural projects
**Resources:** Medium to high, optimized for building
**Features:** Creative mode with building-focused plugins

```toml
# creative_building.toml

[server]
server_name = "Creative Builders Paradise"
motd = "Unleash your creativity!"
max_players = 30
max_connections = 40
view_distance = 16
simulation_distance = 12
max_render_distance = 20
difficulty = "Peaceful"
gamemode = "Creative"
hardcore = false
pvp = false
online_mode = true
whitelist = false
enforce_whitelist = false

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 40
timeout_seconds = 60
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 512

[world]
world_name = "creative_world"
level_path = "worlds/creative_world"
seed = 999888777
generate_structures = true
generator_settings = "{\"biome_size\":6,\"river_size\":6}"
level_type = "default"
spawn_protection = 100
max_world_size = 29999984

[features]
vanilla_mobs = false
redstone = true
world_generation = true
creative_mode = true
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 6
component_capacity = 50000
entity_capacity = 20000
parallel_systems = true
batch_size = 200

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false
max_plugins = 30

# Building-focused plugins
[plugins.world_edit]
enabled = true
max_selection_size = 1000000
history_size = 50

[plugins.plot_system]
enabled = true
plot_size = 100
max_plots_per_player = 5
auto_clear_inactive = true

[plugins.building_tools]
enabled = true
enable_flying = true
unlimited_blocks = true
instant_break = true

[plugins.showcase]
enabled = true
allow_voting = true
featured_builds = true

[monitoring]
enabled = true
metrics_interval = "30s"
performance_alerts = true
detailed_profiling = false

[performance]
thread_pool_size = 6
worker_threads = 3
io_threads = 2
memory_pool_size = 2000
memory_limit = "16GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 2000
chunk_cache_ttl = "600s"
enable_entity_cache = true
entity_cache_size = 10000
entity_cache_ttl = "300s"

[io]
async_world_loading = true
world_save_interval = "300s"
batch_world_saves = true
compression_level = 6

[security]
audit_logging = true
rate_limiting = false
ddos_protection = true

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = false
```

## Survival Hardcore Server

**Use Case:** Challenging survival gameplay
**Resources:** Medium, optimized for survival mechanics
**Features:** Hardcore survival with enhanced difficulty

```toml
# survival_hardcore.toml

[server]
server_name = "Hardcore Survival Challenge"
motd = "Only the strong survive!"
max_players = 25
max_connections = 30
view_distance = 10
simulation_distance = 8
max_render_distance = 12
difficulty = "Hard"
gamemode = "Survival"
hardcore = true
pvp = true
online_mode = true
whitelist = true
enforce_whitelist = true

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 30
timeout_seconds = 30
encryption_enabled = true

[world]
world_name = "hardcore_world"
level_path = "worlds/hardcore_world"
seed = 666777888
generate_structures = true
generator_settings = "{\"biome_size\":4,\"river_size\":4,\"ore_size\":6}"
level_type = "default"
spawn_protection = 0
max_world_size = 20000000

[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = false
command_system = false
performance_monitoring = true
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 4
component_capacity = 15000
entity_capacity = 8000
parallel_systems = true
batch_size = 100

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false
max_plugins = 15

# Hardcore survival plugins
[plugins.enhanced_difficulty]
enabled = true
mob_spawn_multiplier = 2.0
mob_health_multiplier = 1.5
reduced_food_healing = true

[plugins.death_consequences]
enabled = true
drop_all_items = true
lose_experience = true
respawn_delay = 300

[plugins.weather_effects]
enabled = true
realistic_weather = true
weather_damage = true
seasonal_changes = true

[plugins.limited_resources]
enabled = true
ore_scarcity = true
tree_growth_delay = true
crop_growth_delay = true

[monitoring]
enabled = true
metrics_interval = "60s"
performance_alerts = true
detailed_profiling = false

[performance]
thread_pool_size = 4
worker_threads = 2
io_threads = 1
memory_pool_size = 1000
memory_limit = "8GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 800
chunk_cache_ttl = "300s"
enable_entity_cache = true
entity_cache_size = 4000
entity_cache_ttl = "120s"

[security]
audit_logging = true
rate_limiting = true
ddos_protection = true

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = false
```

## Plugin Development Server

**Use Case:** Developing and testing plugins
**Resources:** Development machine
**Features:** Plugin development tools and hot reloading

```toml
# plugin_development.toml

[server]
server_name = "Plugin Dev Server"
motd = "Plugin Development Environment"
max_players = 3
max_connections = 5
view_distance = 8
simulation_distance = 6
max_render_distance = 10
difficulty = "Peaceful"
gamemode = "Creative"
hardcore = false
pvv = false
online_mode = false
whitelist = false
enforce_whitelist = false

[network]
port = 19134
ipv4_addr = "127.0.0.1"
max_clients = 5
timeout_seconds = 120
encryption_enabled = false

[world]
world_name = "plugin_test_world"
level_path = "worlds/plugin_test_world"
seed = 11111
generate_structures = false
generator_settings = "{}"
level_type = "flat"
spawn_protection = 0
max_world_size = 1000000

[features]
vanilla_mobs = true
redstone = true
world_generation = false
creative_mode = true
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

[ecs]
enabled = true
system_thread_count = 2
component_capacity = 10000
entity_capacity = 5000
parallel_systems = false
batch_size = 50

[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = true
max_plugins = 200
plugin_thread_pool = 2

# Development tools
[plugins.plugin_manager]
enabled = true
allow_runtime_loading = true
allow_runtime_unloading = true
plugin_debugging = true

[plugins.test_framework]
enabled = true
auto_run_tests = true
test_timeout = "60s"
generate_test_reports = true

[plugins.performance_profiler]
enabled = true
profile_all_plugins = true
generate_performance_reports = true

[plugins.debug_console]
enabled = true
remote_access = true
console_port = 8081

[monitoring]
enabled = true
metrics_interval = "1s"
performance_alerts = false
detailed_profiling = true
memory_tracking = true
cpu_tracking = true

[monitoring.logging]
log_level = "trace"
log_file = "plugin_dev.log"
rotate_logs = false
max_log_size = "1GB"

[performance]
thread_pool_size = 2
worker_threads = 1
io_threads = 1
memory_pool_size = 500
memory_limit = "4GB"

[caching]
enable_chunk_cache = false
enable_entity_cache = false
enable_packet_cache = false

[io]
async_world_loading = false
world_save_interval = "30s"
batch_world_saves = false
compression_level = 1

[security]
audit_logging = false
rate_limiting = false
ddos_protection = false

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = true

[development]
enable_debug_commands = true
allow_unsafe_operations = true
skip_validation = true
verbose_errors = true
enable_hot_reload = true
auto_restart_on_crash = true
```

## Load Testing Server

**Use Case:** Performance testing and benchmarking
**Resources:** High-end hardware for stress testing
**Features:** Optimized for handling maximum load

```toml
# load_testing.toml

[server]
server_name = "Load Test Server"
motd = "Performance Testing Environment"
max_players = 500
max_connections = 600
view_distance = 6
simulation_distance = 3
max_render_distance = 8
difficulty = "Easy"
gamemode = "Survival"
hardcore = false
pvp = false
online_mode = false
whitelist = false
enforce_whitelist = false

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 600
timeout_seconds = 10
encryption_enabled = false

[network.compression]
algorithm = "flate"
threshold = 64

[network.throttling]
enabled = false

[network.optimization]
tcp_nodelay = true
socket_buffer_size = 524288
send_buffer_size = 262144
receive_buffer_size = 262144
enable_packet_batching = true
batch_size = 100
batch_timeout = "1ms"

[world]
world_name = "load_test_world"
level_path = "worlds/load_test_world"
seed = 999999999
generate_structures = false
generator_settings = "{}"
level_type = "flat"
spawn_protection = 0
max_world_size = 1000000

[features]
vanilla_mobs = false
redstone = false
world_generation = false
creative_mode = false
command_system = false
performance_monitoring = true
ecs_system = true
plugin_system = false

[ecs]
enabled = true
system_thread_count = 32
component_capacity = 500000
entity_capacity = 200000
parallel_systems = true
batch_size = 1000

[plugins]
enabled = false

[monitoring]
enabled = true
metrics_interval = "1s"
performance_alerts = true
detailed_profiling = true
memory_tracking = true
cpu_tracking = true

[monitoring.alerts]
cpu_threshold = 95.0
memory_threshold = 95.0
tps_threshold = 15.0
latency_threshold = "200ms"

[performance]
thread_pool_size = 32
worker_threads = 16
io_threads = 8
memory_pool_size = 50000
memory_limit = "64GB"

[caching]
enable_chunk_cache = true
chunk_cache_size = 10000
chunk_cache_ttl = "3600s"
enable_entity_cache = true
entity_cache_size = 100000
entity_cache_ttl = "1800s"
enable_packet_cache = true
packet_cache_size = 10000

[io]
async_world_loading = true
world_save_interval = "30s"
batch_world_saves = true
compression_level = 1

[security]
audit_logging = false
rate_limiting = false
ddos_protection = false

[mirai]
enable_legacy_api = false
preserve_existing_behavior = false
migration_mode = false

# Load testing specific settings
[load_testing]
enable_bot_simulation = true
bot_count = 400
bot_behavior = "random_movement"
stress_test_mode = true
disable_world_saving = true
minimal_logging = true
```

## Configuration Selection Guide

### Choosing the Right Configuration

1. **Small Private Server**: 5-10 players, friends/family
2. **Medium Community Server**: 20-50 players, community features
3. **Large Public Server**: 100+ players, full optimization
4. **High-Performance Gaming**: Competitive, low-latency focus
5. **Development/Testing**: Plugin development, debugging tools
6. **Minimal Resource**: Limited hardware, basic features
7. **Creative Building**: Creative mode, building tools
8. **Survival Hardcore**: Challenging survival gameplay
9. **Plugin Development**: Plugin testing, hot reloading
10. **Load Testing**: Performance testing, stress testing

### Customization Tips

1. **Adjust player limits** based on your hardware
2. **Modify view/simulation distance** for performance
3. **Enable/disable features** based on your needs
4. **Configure plugins** for your specific use case
5. **Tune caching settings** for your memory constraints
6. **Adjust thread counts** to match your CPU cores
7. **Set appropriate memory limits** for your system

### Migration Between Configurations

```bash
# Backup current configuration
cp unified_config.toml unified_config.toml.backup

# Copy new configuration
cp docs/examples/medium_community_server.toml unified_config.toml

# Migrate world and plugin settings
mirai migrate-config --from unified_config.toml.backup --to unified_config.toml --merge

# Validate new configuration
mirai validate-config --config unified_config.toml
```

These example configurations provide starting points for various server setups. Customize them based on your specific requirements, hardware capabilities, and player needs.