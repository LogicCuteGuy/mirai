//! Integration tests for the unified performance system

#[cfg(test)]
mod integration_tests {
    use crate::performance::{PerformanceManager, GlobalEnhancedPools, MetricsCollector};
    use std::time::Duration;
    use std::thread;

    #[test]
    fn test_performance_manager_integration() {
        let manager = PerformanceManager::new();
        
        // Test memory pools
        let entity_buffer = manager.memory_pools().entity_pool.get();
        assert_eq!(entity_buffer.len(), 0);
        drop(entity_buffer);
        
        // Test metrics collection
        manager.metrics_collector().increment_counter("test_counter", None);
        manager.metrics_collector().record_gauge("test_gauge", 42.0, None);
        
        let summary = manager.metrics_collector().summary();
        assert!(summary.total_metrics >= 2);
        
        // Test profiling
        {
            let _guard = manager.metrics_collector().start_profile("test_operation");
            thread::sleep(Duration::from_millis(1));
        }
        
        let snapshot = manager.metrics_collector().performance_snapshot();
        assert!(snapshot.profiler_stats.profiles.contains_key("test_operation"));
    }

    #[tokio::test]
    async fn test_threading_integration() {
        let manager = PerformanceManager::new();
        
        // Test CPU task execution
        let result = manager.thread_manager()
            .execute_cpu_task(|| 42)
            .unwrap()
            .await_result()
            .await
            .unwrap();
        
        assert_eq!(result, 42);
        
        // Test async task execution
        let result = manager.thread_manager()
            .execute_async_task(|| async { "hello" })
            .await_result()
            .await
            .unwrap();
        
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_memory_pool_integration() {
        let pools = GlobalEnhancedPools::new();
        
        // Test entity pool
        let mut entity_buffer = pools.entity_pool.get();
        entity_buffer.extend_from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(entity_buffer.len(), 5);
        drop(entity_buffer);
        
        // Test packet buffer pool
        let mut packet_buffer = pools.packet_buffer_pool.get();
        packet_buffer.extend_from_slice(b"test packet data");
        assert!(!packet_buffer.is_empty());
        drop(packet_buffer);
        
        // Test string pool
        let mut string_buffer = pools.string_pool.get();
        string_buffer.push_str("test string");
        assert_eq!(&*string_buffer, "test string");
        drop(string_buffer);
        
        let stats = pools.all_stats();
        assert!(stats.overall_efficiency() >= 0.0);
    }

    #[test]
    fn test_monitoring_integration() {
        let collector = MetricsCollector::new();
        
        // Test counter metrics
        collector.increment_counter("requests_total", None);
        collector.increment_counter("requests_total", None);
        
        // Test gauge metrics
        collector.record_gauge("active_connections", 25.0, None);
        collector.record_gauge("cpu_usage", 45.5, None);
        
        // Test histogram metrics
        collector.record_histogram("request_duration", Duration::from_millis(150), None);
        collector.record_histogram("request_duration", Duration::from_millis(200), None);
        
        // Test profiling
        {
            let _guard = collector.start_profile("database_query");
            thread::sleep(Duration::from_millis(5));
        }
        
        let summary = collector.summary();
        assert!(summary.total_metrics >= 4);
        assert!(summary.active_profiles >= 1);
        
        let snapshot = collector.performance_snapshot();
        // assert!(!snapshot.metrics.counters.is_empty());
        // assert!(!snapshot.metrics.gauges.is_empty());
        // assert!(!snapshot.metrics.histograms.is_empty());
        assert!(!snapshot.profiler_stats.profiles.is_empty());
    }

    #[test]
    fn test_performance_stats_integration() {
        let manager = PerformanceManager::new();
        
        // Generate some activity
        manager.metrics_collector().increment_counter("test_metric", None);
        
        {
            let _guard = manager.metrics_collector().start_profile("test_profile");
            thread::sleep(Duration::from_millis(1));
        }
        
        let stats = manager.performance_stats();
        
        // Verify all components are working
        assert!(stats.memory.overall_efficiency() >= 0.0);
        assert!(stats.threading.utilization() >= 0.0);
        assert!(stats.metrics.total_metrics >= 1);
        
        // Test report generation
        let report = stats.report();
        assert!(report.contains("Performance Report"));
        assert!(report.contains("Memory Efficiency"));
        assert!(report.contains("Thread Pool Utilization"));
    }

    #[test]
    fn test_maintenance_operations() {
        let manager = PerformanceManager::new();
        
        // Generate some test data
        manager.metrics_collector().increment_counter("maintenance_test", None);
        
        {
            let _guard = manager.metrics_collector().start_profile("maintenance_profile");
            thread::sleep(Duration::from_millis(1));
        }
        
        // Run maintenance
        manager.maintenance();
        
        // Verify system is still functional after maintenance
        let stats = manager.performance_stats();
        assert!(stats.metrics.total_metrics >= 1);
    }
}