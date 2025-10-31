# Deployment Guide: Migrating from Standard Mirai

This comprehensive guide walks you through deploying the unified Mirai server and migrating from a standard Mirai installation. The unified server maintains full backward compatibility while adding advanced features.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation Methods](#installation-methods)
3. [Migration Process](#migration-process)
4. [Configuration Migration](#configuration-migration)
5. [Plugin Migration](#plugin-migration)
6. [World Migration](#world-migration)
7. [Performance Optimization](#performance-optimization)
8. [Monitoring and Maintenance](#monitoring-and-maintenance)
9. [Rollback Procedures](#rollback-procedures)
10. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

**Minimum Requirements:**
- CPU: 2 cores, 2.4 GHz
- RAM: 4 GB
- Storage: 10 GB free space
- Network: Stable internet connection
- OS: Windows 10/11, Linux (Ubuntu 20.04+), macOS 10.15+

**Recommended Requirements:**
- CPU: 4+ cores, 3.0+ GHz
- RAM: 8+ GB
- Storage: 50+ GB SSD
- Network: High-speed internet (100+ Mbps)
- OS: Latest stable versions

### Software Dependencies

```bash
# Rust toolchain (required for building from source)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Git (for cloning repository)
# Windows: Download from https://git-scm.com/
# Linux: sudo apt install git
# macOS: xcode-select --install

# Optional: Docker (for containerized deployment)
# Follow instructions at https://docs.docker.com/get-docker/
```

## Installation Methods

### Method 1: Pre-built Binaries (Recommended)

```bash
# Download the latest release
wget https://github.com/mirai-rs/mirai/releases/latest/download/mirai-unified-linux-x64.tar.gz

# Extract
tar -xzf mirai-unified-linux-x64.tar.gz

# Make executable
chmod +x mirai-unified

# Run
./mirai-unified --config unified_config.toml
```

### Method 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/mirai-rs/mirai.git
cd mirai

# Build with unified features
cargo build --release --features "unified,ecs,plugins,monitoring"

# The binary will be in target/release/mirai
```

### Method 3: Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --features "unified,ecs,plugins,monitoring"

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mirai /usr/local/bin/mirai
COPY --from=builder /app/unified_config_example.toml /etc/mirai/config.toml

EXPOSE 19132/udp
VOLUME ["/data"]

CMD ["mirai", "--config", "/etc/mirai/config.toml", "--world-path", "/data"]
```

```bash
# Build and run
docker build -t mirai-unified .
docker run -d -p 19132:19132/udp -v ./worlds:/data mirai-unified
```

## Migration Process

### Step 1: Backup Current Installation

```bash
#!/bin/bash
# backup_mirai.sh

BACKUP_DIR="mirai_backup_$(date +%Y%m%d_%H%M%S)"
MIRAI_DIR="/path/to/your/mirai"

echo "Creating backup in $BACKUP_DIR..."

mkdir -p "$BACKUP_DIR"

# Backup configuration
cp "$MIRAI_DIR/mirai.toml" "$BACKUP_DIR/" 2>/dev/null || echo "No mirai.toml found"
cp "$MIRAI_DIR/server.properties" "$BACKUP_DIR/" 2>/dev/null || echo "No server.properties found"

# Backup worlds
cp -r "$MIRAI_DIR/worlds" "$BACKUP_DIR/" 2>/dev/null || echo "No worlds directory found"

# Backup plugins
cp -r "$MIRAI_DIR/plugins" "$BACKUP_DIR/" 2>/dev/null || echo "No plugins directory found"

# Backup logs
cp -r "$MIRAI_DIR/logs" "$BACKUP_DIR/" 2>/dev/null || echo "No logs directory found"

echo "Backup completed: $BACKUP_DIR"
```

### Step 2: Stop Current Server

```bash
# Gracefully stop the current Mirai server
# Method 1: If running as a service
sudo systemctl stop mirai

# Method 2: If running in screen/tmux
screen -S mirai -X stuff "stop$(echo -ne '\r')"

# Method 3: Send SIGTERM to process
pkill -TERM mirai
```

### Step 3: Install Unified Server

```bash
# Download and install unified server
wget https://github.com/mirai-rs/mirai/releases/latest/download/mirai-unified-linux-x64.tar.gz
tar -xzf mirai-unified-linux-x64.tar.gz

# Replace existing binary
sudo cp mirai-unified /usr/local/bin/mirai
sudo chmod +x /usr/local/bin/mirai

# Verify installation
mirai --version
```

### Step 4: Migrate Configuration

```bash
# Use the built-in migration tool
mirai migrate-config --from mirai.toml --to unified_config.toml

# Or manually migrate (see Configuration Migration section)
```

## Configuration Migration

### Automatic Migration

The unified server includes a configuration migration tool:

```bash
# Migrate from standard Mirai configuration
mirai migrate-config \
    --from /path/to/mirai.toml \
    --to /path/to/unified_config.toml \
    --backup \
    --validate

# Options:
# --backup: Create backup of original config
# --validate: Validate migrated configuration
# --dry-run: Show what would be migrated without making changes
```

### Manual Migration

#### Standard Mirai Configuration

```toml
# mirai.toml (old format)
[server]
name = "My Mirai Server"
port = 19132
max_players = 50

[world]
name = "world"
path = "worlds/world"

[network]
compression = true
encryption = true
```

#### Unified Configuration

```toml
# unified_config.toml (new format)
[server]
server_name = "My Mirai Server"
motd = "Upgraded to Unified Mirai!"
max_players = 50
max_connections = 100
view_distance = 12
simulation_distance = 8
difficulty = "Hard"
gamemode = "Survival"
pvp = true
online_mode = true

[network]
port = 19132
ipv4_addr = "0.0.0.0"
max_clients = 100
timeout_seconds = 30
encryption_enabled = true

[network.compression]
algorithm = "flate"
threshold = 256

[world]
world_name = "world"
level_path = "worlds/world"
seed = 12345
generate_structures = true
level_type = "default"
spawn_protection = 16

# New unified features
[features]
vanilla_mobs = true
redstone = true
world_generation = true
creative_mode = true
command_system = true
performance_monitoring = true
ecs_system = true
plugin_system = true

[mirai]
enable_legacy_api = true
preserve_existing_behavior = true
migration_mode = false

# ECS configuration
[ecs]
enabled = true
system_thread_count = 4
component_capacity = 10000

# Plugin system
[plugins]
enabled = true
plugin_directory = "plugins"
hot_reload = false

# Performance monitoring
[monitoring]
enabled = true
metrics_interval = "10s"
performance_alerts = true
detailed_profiling = false

# Advanced features
[codegen]
enabled = true
auto_update_protocol = false
generate_components = true

[security]
audit_logging = true
rate_limiting = true
ddos_protection = true
```

### Configuration Validation

```bash
# Validate configuration before starting
mirai validate-config --config unified_config.toml

# Check for common issues
mirai doctor --config unified_config.toml
```

## Plugin Migration

### Compatible Plugins

Most existing Mirai plugins are compatible with the unified server:

```bash
# Check plugin compatibility
mirai check-plugins --plugin-dir plugins/

# Output example:
# ✓ economy_plugin.so - Compatible
# ✓ protection_plugin.so - Compatible  
# ⚠ custom_plugin.so - Needs update (ECS features available)
# ✗ old_plugin.so - Incompatible (deprecated API)
```

### Plugin Directory Structure

```
plugins/
├── compatible/          # Existing compatible plugins
├── enhanced/           # Plugins with ECS enhancements
├── legacy/            # Legacy plugins (compatibility mode)
└── disabled/          # Disabled plugins
```

### Plugin Migration Script

```bash
#!/bin/bash
# migrate_plugins.sh

PLUGIN_DIR="plugins"
BACKUP_DIR="plugins_backup_$(date +%Y%m%d_%H%M%S)"

echo "Backing up plugins..."
cp -r "$PLUGIN_DIR" "$BACKUP_DIR"

echo "Checking plugin compatibility..."
for plugin in "$PLUGIN_DIR"/*.so; do
    if [ -f "$plugin" ]; then
        echo "Checking $(basename "$plugin")..."
        if mirai check-plugin --plugin "$plugin"; then
            echo "  ✓ Compatible"
        else
            echo "  ⚠ Moving to legacy directory"
            mkdir -p "$PLUGIN_DIR/legacy"
            mv "$plugin" "$PLUGIN_DIR/legacy/"
        fi
    fi
done

echo "Plugin migration completed"
```

## World Migration

### LevelDB World Compatibility

The unified server maintains full compatibility with existing Mirai worlds:

```bash
# Verify world compatibility
mirai check-world --world-path worlds/world

# Migrate world if needed (usually not required)
mirai migrate-world --from worlds/world --to worlds/world_unified --backup
```

### World Backup and Validation

```bash
#!/bin/bash
# backup_world.sh

WORLD_PATH="worlds/world"
BACKUP_PATH="worlds/backups/world_$(date +%Y%m%d_%H%M%S)"

echo "Creating world backup..."
mkdir -p "$(dirname "$BACKUP_PATH")"
cp -r "$WORLD_PATH" "$BACKUP_PATH"

echo "Validating world integrity..."
mirai validate-world --world-path "$WORLD_PATH"

echo "World backup completed: $BACKUP_PATH"
```

## Performance Optimization

### Initial Performance Configuration

```toml
# unified_config.toml - Performance optimized
[server]
max_players = 100
view_distance = 10
simulation_distance = 6

[network]
max_clients = 120
timeout_seconds = 30

[network.compression]
algorithm = "flate"
threshold = 256

[features]
performance_monitoring = true

[ecs]
enabled = true
system_thread_count = 8  # Adjust based on CPU cores
component_capacity = 20000

[monitoring]
enabled = true
metrics_interval = "5s"
performance_alerts = true
detailed_profiling = true

[performance]
# Thread pool configuration
thread_pool_size = 8
worker_threads = 4

# Memory management
memory_pool_size = 1000
gc_interval = "30s"

# Caching
enable_chunk_cache = true
chunk_cache_size = 1000
enable_entity_cache = true
entity_cache_size = 5000
```

### Performance Monitoring Setup

```bash
# Enable performance monitoring
mirai monitor --config unified_config.toml --output performance.log

# Real-time performance dashboard
mirai dashboard --port 8080 --config unified_config.toml
```

### Optimization Script

```bash
#!/bin/bash
# optimize_performance.sh

CONFIG_FILE="unified_config.toml"

echo "Optimizing Mirai configuration for this system..."

# Detect system resources
CPU_CORES=$(nproc)
TOTAL_RAM=$(free -m | awk 'NR==2{printf "%.0f", $2/1024}')

echo "Detected: $CPU_CORES CPU cores, ${TOTAL_RAM}GB RAM"

# Update configuration based on system resources
sed -i "s/system_thread_count = .*/system_thread_count = $CPU_CORES/" "$CONFIG_FILE"
sed -i "s/thread_pool_size = .*/thread_pool_size = $CPU_CORES/" "$CONFIG_FILE"

# Adjust memory settings
if [ "$TOTAL_RAM" -gt 8 ]; then
    sed -i "s/component_capacity = .*/component_capacity = 50000/" "$CONFIG_FILE"
    sed -i "s/chunk_cache_size = .*/chunk_cache_size = 2000/" "$CONFIG_FILE"
elif [ "$TOTAL_RAM" -gt 4 ]; then
    sed -i "s/component_capacity = .*/component_capacity = 20000/" "$CONFIG_FILE"
    sed -i "s/chunk_cache_size = .*/chunk_cache_size = 1000/" "$CONFIG_FILE"
else
    sed -i "s/component_capacity = .*/component_capacity = 10000/" "$CONFIG_FILE"
    sed -i "s/chunk_cache_size = .*/chunk_cache_size = 500/" "$CONFIG_FILE"
fi

echo "Configuration optimized for your system"
```

## Monitoring and Maintenance

### System Service Setup

```ini
# /etc/systemd/system/mirai-unified.service
[Unit]
Description=Mirai Unified Minecraft Server
After=network.target

[Service]
Type=simple
User=mirai
Group=mirai
WorkingDirectory=/opt/mirai
ExecStart=/usr/local/bin/mirai --config /opt/mirai/unified_config.toml
ExecStop=/bin/kill -TERM $MAINPID
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable mirai-unified
sudo systemctl start mirai-unified

# Check status
sudo systemctl status mirai-unified
```

### Monitoring Scripts

```bash
#!/bin/bash
# monitor_mirai.sh

LOG_FILE="/var/log/mirai/monitoring.log"
CONFIG_FILE="/opt/mirai/unified_config.toml"

while true; do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Check if server is running
    if pgrep -f "mirai.*unified_config.toml" > /dev/null; then
        STATUS="RUNNING"
        
        # Get performance metrics
        MEMORY=$(ps -o pid,vsz,rss,comm -p $(pgrep -f mirai) | tail -1 | awk '{print $3}')
        CPU=$(ps -o pid,pcpu,comm -p $(pgrep -f mirai) | tail -1 | awk '{print $2}')
        
        echo "$TIMESTAMP - $STATUS - Memory: ${MEMORY}KB, CPU: ${CPU}%" >> "$LOG_FILE"
    else
        STATUS="STOPPED"
        echo "$TIMESTAMP - $STATUS - Server not running!" >> "$LOG_FILE"
        
        # Optionally restart
        # sudo systemctl start mirai-unified
    fi
    
    sleep 60
done
```

### Log Rotation

```bash
# /etc/logrotate.d/mirai-unified
/var/log/mirai/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 mirai mirai
    postrotate
        systemctl reload mirai-unified
    endscript
}
```

## Rollback Procedures

### Quick Rollback

```bash
#!/bin/bash
# rollback_mirai.sh

BACKUP_DIR="$1"

if [ -z "$BACKUP_DIR" ]; then
    echo "Usage: $0 <backup_directory>"
    exit 1
fi

echo "Rolling back to backup: $BACKUP_DIR"

# Stop unified server
sudo systemctl stop mirai-unified

# Restore configuration
cp "$BACKUP_DIR/mirai.toml" /opt/mirai/ 2>/dev/null || echo "No config to restore"

# Restore worlds
rm -rf /opt/mirai/worlds
cp -r "$BACKUP_DIR/worlds" /opt/mirai/ 2>/dev/null || echo "No worlds to restore"

# Restore plugins
rm -rf /opt/mirai/plugins
cp -r "$BACKUP_DIR/plugins" /opt/mirai/ 2>/dev/null || echo "No plugins to restore"

# Restore original binary (if backed up)
if [ -f "$BACKUP_DIR/mirai_original" ]; then
    sudo cp "$BACKUP_DIR/mirai_original" /usr/local/bin/mirai
    sudo chmod +x /usr/local/bin/mirai
fi

# Start server with original configuration
sudo systemctl start mirai

echo "Rollback completed"
```

### Gradual Rollback

```bash
# Disable unified features gradually
mirai --config unified_config.toml --disable-ecs --disable-plugins

# Or modify configuration
sed -i 's/ecs_system = true/ecs_system = false/' unified_config.toml
sed -i 's/plugin_system = true/plugin_system = false/' unified_config.toml
```

## Troubleshooting

### Common Issues

#### 1. Configuration Errors

```bash
# Problem: Invalid configuration
# Solution: Validate and fix configuration
mirai validate-config --config unified_config.toml --verbose

# Common fixes:
# - Check TOML syntax
# - Verify file paths exist
# - Ensure port is not in use
```

#### 2. Plugin Compatibility

```bash
# Problem: Plugin fails to load
# Solution: Check plugin compatibility
mirai check-plugin --plugin plugins/problematic_plugin.so --verbose

# Workarounds:
# - Move to legacy directory
# - Disable plugin temporarily
# - Check for plugin updates
```

#### 3. Performance Issues

```bash
# Problem: High CPU/memory usage
# Solution: Optimize configuration
mirai doctor --config unified_config.toml --performance

# Adjustments:
# - Reduce view distance
# - Lower thread count
# - Disable detailed profiling
```

#### 4. World Loading Issues

```bash
# Problem: World fails to load
# Solution: Validate world integrity
mirai validate-world --world-path worlds/world --repair

# If repair fails:
# - Restore from backup
# - Create new world
# - Check disk space
```

### Diagnostic Commands

```bash
# System information
mirai system-info

# Configuration check
mirai doctor --config unified_config.toml

# Performance analysis
mirai analyze-performance --log-file server.log

# Network diagnostics
mirai test-network --port 19132

# Plugin diagnostics
mirai plugin-info --plugin-dir plugins/
```

### Log Analysis

```bash
# Search for errors
grep -i error /var/log/mirai/server.log | tail -20

# Performance issues
grep -i "performance\|slow\|lag" /var/log/mirai/server.log

# Plugin issues
grep -i "plugin" /var/log/mirai/server.log | grep -i "error\|fail"

# Memory issues
grep -i "memory\|oom" /var/log/mirai/server.log
```

### Getting Help

1. **Documentation**: Check the official documentation
2. **GitHub Issues**: Search existing issues or create new ones
3. **Community Discord**: Join the Mirai community
4. **Logs**: Always include relevant log files when asking for help

### Emergency Contacts

```bash
# Create emergency recovery script
cat > emergency_recovery.sh << 'EOF'
#!/bin/bash
echo "Emergency Mirai Recovery"
echo "1. Stop server"
sudo systemctl stop mirai-unified

echo "2. Check system resources"
df -h
free -h
ps aux | grep mirai

echo "3. Restore from latest backup"
LATEST_BACKUP=$(ls -t mirai_backup_* | head -1)
echo "Latest backup: $LATEST_BACKUP"

echo "4. Manual recovery steps:"
echo "   - Restore config: cp $LATEST_BACKUP/mirai.toml /opt/mirai/"
echo "   - Restore worlds: cp -r $LATEST_BACKUP/worlds /opt/mirai/"
echo "   - Start server: sudo systemctl start mirai"
EOF

chmod +x emergency_recovery.sh
```

This deployment guide provides comprehensive instructions for migrating from standard Mirai to the unified server. Follow the steps carefully and always maintain backups during the migration process.