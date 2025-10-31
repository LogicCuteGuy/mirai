#!/usr/bin/env python3
"""
Load testing script for the merged mirai system.
Simulates multiple concurrent connections to test server performance.
"""

import asyncio
import aiohttp
import json
import time
import argparse
import sys
from typing import List, Dict, Any
from dataclasses import dataclass, asdict
from concurrent.futures import ThreadPoolExecutor
import statistics

@dataclass
class ConnectionResult:
    connection_id: int
    connect_time: float
    total_time: float
    packets_sent: int
    packets_received: int
    errors: int
    success: bool

@dataclass
class LoadTestResults:
    total_connections: int
    successful_connections: int
    failed_connections: int
    total_duration: float
    avg_connect_time: float
    avg_response_time: float
    packets_per_second: float
    errors_per_second: float
    memory_usage_mb: float

class MinecraftLoadTester:
    def __init__(self, host: str = "localhost", port: int = 19132):
        self.host = host
        self.port = port
        self.results: List[ConnectionResult] = []
    
    async def simulate_bedrock_connection(self, connection_id: int, duration: int) -> ConnectionResult:
        """Simulate a Bedrock client connection."""
        start_time = time.time()
        connect_time = 0
        packets_sent = 0
        packets_received = 0
        errors = 0
        success = False
        
        try:
            # Simulate RakNet handshake
            connect_start = time.time()
            
            # In a real implementation, this would use actual RakNet protocol
            # For now, we'll simulate with HTTP requests to test the server
            async with aiohttp.ClientSession() as session:
                # Simulate connection establishment
                try:
                    async with session.get(f"http://{self.host}:8080/health") as response:
                        if response.status == 200:
                            connect_time = time.time() - connect_start
                            success = True
                except Exception as e:
                    errors += 1
                    print(f"Connection {connection_id} failed to connect: {e}")
                
                if success:
                    # Simulate packet exchange
                    end_time = start_time + duration
                    while time.time() < end_time and errors < 10:
                        try:
                            # Simulate sending packets
                            packet_data = {
                                "type": "keep_alive",
                                "timestamp": time.time(),
                                "connection_id": connection_id
                            }
                            
                            async with session.post(
                                f"http://{self.host}:8080/packet",
                                json=packet_data
                            ) as response:
                                if response.status == 200:
                                    packets_sent += 1
                                    packets_received += 1
                                else:
                                    errors += 1
                            
                            # Wait between packets
                            await asyncio.sleep(0.1)
                            
                        except Exception as e:
                            errors += 1
                            if errors >= 10:
                                break
        
        except Exception as e:
            errors += 1
            print(f"Connection {connection_id} encountered error: {e}")
        
        total_time = time.time() - start_time
        
        return ConnectionResult(
            connection_id=connection_id,
            connect_time=connect_time,
            total_time=total_time,
            packets_sent=packets_sent,
            packets_received=packets_received,
            errors=errors,
            success=success and errors < 10
        )
    
    async def run_load_test(self, num_connections: int, duration: int) -> LoadTestResults:
        """Run the load test with specified parameters."""
        print(f"Starting load test: {num_connections} connections for {duration} seconds")
        
        start_time = time.time()
        
        # Create connection tasks
        tasks = []
        for i in range(num_connections):
            task = asyncio.create_task(
                self.simulate_bedrock_connection(i, duration)
            )
            tasks.append(task)
            
            # Stagger connection attempts to avoid overwhelming the server
            if i % 10 == 0 and i > 0:
                await asyncio.sleep(0.1)
        
        # Wait for all connections to complete
        self.results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # Filter out exceptions
        valid_results = [r for r in self.results if isinstance(r, ConnectionResult)]
        
        total_duration = time.time() - start_time
        
        # Calculate statistics
        successful = [r for r in valid_results if r.success]
        failed = [r for r in valid_results if not r.success]
        
        avg_connect_time = statistics.mean([r.connect_time for r in successful]) if successful else 0
        avg_response_time = statistics.mean([r.total_time for r in successful]) if successful else 0
        
        total_packets = sum(r.packets_sent for r in valid_results)
        packets_per_second = total_packets / total_duration if total_duration > 0 else 0
        
        total_errors = sum(r.errors for r in valid_results)
        errors_per_second = total_errors / total_duration if total_duration > 0 else 0
        
        return LoadTestResults(
            total_connections=len(valid_results),
            successful_connections=len(successful),
            failed_connections=len(failed),
            total_duration=total_duration,
            avg_connect_time=avg_connect_time,
            avg_response_time=avg_response_time,
            packets_per_second=packets_per_second,
            errors_per_second=errors_per_second,
            memory_usage_mb=self.get_memory_usage()
        )
    
    def get_memory_usage(self) -> float:
        """Get current memory usage in MB."""
        try:
            import psutil
            process = psutil.Process()
            return process.memory_info().rss / 1024 / 1024
        except ImportError:
            return 0.0
    
    def generate_report(self, results: LoadTestResults) -> str:
        """Generate a detailed load test report."""
        report = [
            "# Load Test Report\n",
            f"## Test Configuration",
            f"- **Target**: {self.host}:{self.port}",
            f"- **Total Connections**: {results.total_connections}",
            f"- **Test Duration**: {results.total_duration:.2f} seconds\n",
            
            f"## Results Summary",
            f"- **Successful Connections**: {results.successful_connections}",
            f"- **Failed Connections**: {results.failed_connections}",
            f"- **Success Rate**: {(results.successful_connections / results.total_connections * 100):.1f}%\n",
            
            f"## Performance Metrics",
            f"- **Average Connect Time**: {results.avg_connect_time:.3f} seconds",
            f"- **Average Response Time**: {results.avg_response_time:.3f} seconds",
            f"- **Packets per Second**: {results.packets_per_second:.1f}",
            f"- **Errors per Second**: {results.errors_per_second:.1f}",
            f"- **Memory Usage**: {results.memory_usage_mb:.1f} MB\n"
        ]
        
        # Add performance assessment
        if results.successful_connections / results.total_connections >= 0.95:
            report.append("## ✅ Performance Assessment: PASS")
            report.append("The server handled the load test successfully with minimal failures.")
        elif results.successful_connections / results.total_connections >= 0.80:
            report.append("## ⚠️ Performance Assessment: WARNING")
            report.append("The server showed some performance issues under load.")
        else:
            report.append("## ❌ Performance Assessment: FAIL")
            report.append("The server failed to handle the load test adequately.")
        
        return "\n".join(report)

async def main():
    parser = argparse.ArgumentParser(description="Load test the merged mirai system")
    parser.add_argument("--host", default="localhost", help="Server host")
    parser.add_argument("--port", type=int, default=19132, help="Server port")
    parser.add_argument("--connections", type=int, default=100, help="Number of concurrent connections")
    parser.add_argument("--duration", type=int, default=60, help="Test duration in seconds")
    parser.add_argument("--output", default="load_test_results.json", help="Output file for results")
    
    args = parser.parse_args()
    
    tester = MinecraftLoadTester(args.host, args.port)
    
    try:
        print("Starting load test...")
        results = await tester.run_load_test(args.connections, args.duration)
        
        # Generate and display report
        report = tester.generate_report(results)
        print(report)
        
        # Save results to JSON
        with open(args.output, 'w') as f:
            json.dump(asdict(results), f, indent=2)
        
        # Save report to markdown
        with open("load_test_report.md", 'w') as f:
            f.write(report)
        
        print(f"\nResults saved to {args.output}")
        print(f"Report saved to load_test_report.md")
        
        # Exit with error code if test failed
        success_rate = results.successful_connections / results.total_connections
        if success_rate < 0.80:
            print(f"\n❌ Load test failed with {success_rate:.1%} success rate")
            sys.exit(1)
        else:
            print(f"\n✅ Load test passed with {success_rate:.1%} success rate")
    
    except Exception as e:
        print(f"Load test failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())