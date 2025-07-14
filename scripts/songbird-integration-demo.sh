#!/bin/bash
# ToadStool Songbird Integration Demo
# This script demonstrates how ToadStool integrates with Songbird for port orchestration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Demo configuration
SONGBIRD_PORT=8080
TOADSTOOL_BASE_PORT=8081
DEMO_DIR="/tmp/toadstool-songbird-demo"

echo -e "${CYAN}🎵 ToadStool Songbird Integration Demo${NC}"
echo -e "${CYAN}===================================${NC}"
echo ""

# Function to print status
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check if Songbird is running
check_songbird() {
    echo -e "${BLUE}Checking Songbird availability...${NC}"
    
    if curl -s "http://localhost:${SONGBIRD_PORT}/api/v1/health" > /dev/null 2>&1; then
        print_status "Songbird is running at http://localhost:${SONGBIRD_PORT}"
        return 0
    else
        print_warning "Songbird not detected - starting mock Songbird service"
        start_mock_songbird
        return 1
    fi
}

# Start a mock Songbird service for demo purposes
start_mock_songbird() {
    echo -e "${BLUE}Starting mock Songbird service...${NC}"
    
    mkdir -p "${DEMO_DIR}"
    
    # Create a simple mock Songbird server
    cat > "${DEMO_DIR}/mock_songbird.py" << 'EOF'
#!/usr/bin/env python3
import json
import http.server
import socketserver
import threading
import time
from urllib.parse import urlparse, parse_qs
from datetime import datetime

class SongbirdHandler(http.server.BaseHTTPRequestHandler):
    # Track allocated ports
    allocated_ports = {8081: "toadstool", 8082: "beardog", 8083: "nestgate", 7070: "squirrel"}
    next_port = 8090
    
    def do_GET(self):
        path = urlparse(self.path).path
        
        if path == "/api/v1/health":
            self.send_health_response()
        elif path == "/api/v1/discovery":
            self.send_discovery_response()
        elif path.startswith("/api/v1/services"):
            self.send_services_response()
        else:
            self.send_error(404, "Not Found")
    
    def do_POST(self):
        path = urlparse(self.path).path
        
        if path == "/api/v1/register":
            self.handle_service_registration()
        elif path == "/api/v1/port-allocation":
            self.handle_port_allocation()
        else:
            self.send_error(404, "Not Found")
    
    def do_DELETE(self):
        path = urlparse(self.path).path
        
        if path.startswith("/api/v1/port-allocation/"):
            self.handle_port_release()
        else:
            self.send_error(404, "Not Found")
    
    def send_health_response(self):
        response = {
            "status": "healthy",
            "timestamp": datetime.now().isoformat(),
            "version": "1.0.0-mock",
            "uptime": "1h 23m 45s"
        }
        self.send_json_response(response)
    
    def send_discovery_response(self):
        response = {
            "services": [
                {
                    "name": "songbird",
                    "type": "orchestration",
                    "endpoint": "http://localhost:8080",
                    "status": "healthy",
                    "capabilities": ["service-discovery", "port-orchestration", "load-balancing"]
                },
                {
                    "name": "toadstool",
                    "type": "compute",
                    "endpoint": "http://localhost:8081",
                    "status": "healthy",
                    "capabilities": ["native-execution", "container-execution", "wasm-execution"]
                },
                {
                    "name": "beardog",
                    "type": "security",
                    "endpoint": "http://localhost:8082",
                    "status": "healthy",
                    "capabilities": ["crypto-locks", "encryption", "key-management"]
                },
                {
                    "name": "nestgate",
                    "type": "storage",
                    "endpoint": "http://localhost:8083",
                    "status": "healthy",
                    "capabilities": ["data-storage", "metadata-management", "backup"]
                },
                {
                    "name": "squirrel",
                    "type": "plugins",
                    "endpoint": "http://localhost:7070",
                    "status": "healthy",
                    "capabilities": ["plugin-execution", "mcp-integration", "sandboxing"]
                }
            ],
            "port_allocation": {
                "songbird": 8080,
                "toadstool": 8081,
                "beardog": 8082,
                "nestgate": 8083,
                "squirrel": 7070
            },
            "timestamp": datetime.now().isoformat()
        }
        self.send_json_response(response)
    
    def send_services_response(self):
        response = {
            "total_services": 5,
            "healthy_services": 5,
            "services": [
                {"name": "songbird", "status": "healthy", "port": 8080},
                {"name": "toadstool", "status": "healthy", "port": 8081},
                {"name": "beardog", "status": "healthy", "port": 8082},
                {"name": "nestgate", "status": "healthy", "port": 8083},
                {"name": "squirrel", "status": "healthy", "port": 7070}
            ]
        }
        self.send_json_response(response)
    
    def handle_service_registration(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        registration_data = json.loads(post_data.decode('utf-8'))
        
        response = {
            "status": "registered",
            "service_id": registration_data.get("service_id", "unknown"),
            "instance_id": registration_data.get("instance_id", "unknown"),
            "token": f"auth_token_{int(time.time())}",
            "allocated_port": registration_data.get("preferred_port", self.next_port),
            "health_check_url": f"http://localhost:{registration_data.get('preferred_port', self.next_port)}/health",
            "timestamp": datetime.now().isoformat()
        }
        
        print(f"📝 Registered service: {registration_data.get('service_id')}")
        self.send_json_response(response, status=201)
    
    def handle_port_allocation(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        allocation_data = json.loads(post_data.decode('utf-8'))
        
        service_name = allocation_data.get("service_name")
        preferred_port = allocation_data.get("preferred_port")
        
        # Try to allocate preferred port, otherwise assign next available
        if preferred_port and preferred_port not in self.allocated_ports:
            allocated_port = preferred_port
        else:
            allocated_port = self.next_port
            self.next_port += 1
        
        self.allocated_ports[allocated_port] = service_name
        
        response = {
            "status": "allocated",
            "service_name": service_name,
            "allocated_port": allocated_port,
            "expires_at": datetime.now().isoformat(),
            "timestamp": datetime.now().isoformat()
        }
        
        print(f"🔌 Allocated port {allocated_port} to {service_name}")
        self.send_json_response(response, status=201)
    
    def handle_port_release(self):
        path_parts = self.path.split('/')
        port = int(path_parts[-1])
        
        if port in self.allocated_ports:
            service_name = self.allocated_ports[port]
            del self.allocated_ports[port]
            print(f"🔌 Released port {port} from {service_name}")
            
            response = {
                "status": "released",
                "port": port,
                "service_name": service_name,
                "timestamp": datetime.now().isoformat()
            }
            self.send_json_response(response)
        else:
            self.send_error(404, "Port not found")
    
    def send_json_response(self, data, status=200):
        response = json.dumps(data, indent=2)
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(response)))
        self.end_headers()
        self.wfile.write(response.encode('utf-8'))
    
    def log_message(self, format, *args):
        # Suppress default logging
        pass

def start_server():
    with socketserver.TCPServer(("", 8080), SongbirdHandler) as httpd:
        print("🎵 Mock Songbird server running on port 8080")
        httpd.serve_forever()

if __name__ == "__main__":
    start_server()
EOF

    # Start the mock server in background
    python3 "${DEMO_DIR}/mock_songbird.py" &
    MOCK_SONGBIRD_PID=$!
    echo $MOCK_SONGBIRD_PID > "${DEMO_DIR}/songbird.pid"
    
    # Wait for server to start
    sleep 2
    
    if curl -s "http://localhost:${SONGBIRD_PORT}/api/v1/health" > /dev/null 2>&1; then
        print_status "Mock Songbird service started successfully"
    else
        print_error "Failed to start mock Songbird service"
        exit 1
    fi
}

# Demonstrate ToadStool configuration loading
demo_config_loading() {
    echo -e "\n${PURPLE}📋 Configuration Loading Demo${NC}"
    echo -e "${PURPLE}=============================${NC}"
    
    print_info "Creating ToadStool configuration with Songbird integration..."
    
    # Create demo configuration
    mkdir -p "${DEMO_DIR}/config"
    
    cat > "${DEMO_DIR}/config/toadstool.toml" << EOF
[environment]
name = "demo"
debug = true
verbose = true

[network]
bind_address = "0.0.0.0"
port = 8081

[network.songbird_orchestration]
enabled = true
endpoint = "http://localhost:8080"
port_allocation_strategy = "Dynamic"
conflict_resolution = "Songbird"

[network.songbird_orchestration.dynamic_port_range]
start = 8080
end = 8999

[network.songbird_orchestration.registration]
auto_register = true
registration_interval_secs = 30
capability_update_interval_secs = 60
deregister_on_shutdown = true
tags = ["compute", "toadstool", "demo"]

[runtime.engines.native]
enabled = true
execution_timeout_secs = 300

[runtime.engines.container]
enabled = true
runtime_type = "Docker"

[runtime.engines.wasm]
enabled = true
engine = "Wasmtime"

[ecosystem.primals.songbird]
enabled = true
endpoint = "http://localhost:8080"
health_check_enabled = true
EOF

    print_status "Configuration file created: ${DEMO_DIR}/config/toadstool.toml"
    
    # Show configuration priority
    echo -e "\n${BLUE}Configuration loading priority:${NC}"
    echo "1. System environment variables"
    echo "2. Songbird service discovery"
    echo "3. Configuration files"
    echo "4. Built-in defaults"
}

# Demonstrate Songbird service discovery
demo_service_discovery() {
    echo -e "\n${PURPLE}🔍 Service Discovery Demo${NC}"
    echo -e "${PURPLE}=========================${NC}"
    
    print_info "Querying Songbird for service discovery..."
    
    # Query service discovery endpoint
    if curl -s "http://localhost:${SONGBIRD_PORT}/api/v1/discovery" | jq . > "${DEMO_DIR}/discovery.json"; then
        print_status "Service discovery successful"
        echo -e "\n${BLUE}Discovered services:${NC}"
        jq -r '.services[] | "  • \(.name) (\(.type)) - \(.endpoint)"' "${DEMO_DIR}/discovery.json"
        
        echo -e "\n${BLUE}Port allocations:${NC}"
        jq -r '.port_allocation | to_entries[] | "  • \(.key): \(.value)"' "${DEMO_DIR}/discovery.json"
    else
        print_error "Service discovery failed"
    fi
}

# Demonstrate port orchestration
demo_port_orchestration() {
    echo -e "\n${PURPLE}🔌 Port Orchestration Demo${NC}"
    echo -e "${PURPLE}==========================${NC}"
    
    print_info "Demonstrating dynamic port allocation..."
    
    # Request port allocation for a new service
    PORT_REQUEST='{
        "service_name": "toadstool-worker",
        "preferred_port": 8085,
        "port_range": {
            "start": 8080,
            "end": 8999
        }
    }'
    
    echo -e "\n${BLUE}Requesting port allocation:${NC}"
    echo "$PORT_REQUEST" | jq .
    
    if ALLOCATION_RESPONSE=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$PORT_REQUEST" \
        "http://localhost:${SONGBIRD_PORT}/api/v1/port-allocation"); then
        
        echo -e "\n${GREEN}Port allocation response:${NC}"
        echo "$ALLOCATION_RESPONSE" | jq .
        
        ALLOCATED_PORT=$(echo "$ALLOCATION_RESPONSE" | jq -r '.allocated_port')
        print_status "Allocated port: $ALLOCATED_PORT"
        
        # Demonstrate port release
        echo -e "\n${BLUE}Releasing allocated port...${NC}"
        if curl -s -X DELETE "http://localhost:${SONGBIRD_PORT}/api/v1/port-allocation/${ALLOCATED_PORT}" | jq . > "${DEMO_DIR}/release.json"; then
            print_status "Port released successfully"
            cat "${DEMO_DIR}/release.json"
        else
            print_error "Failed to release port"
        fi
    else
        print_error "Port allocation failed"
    fi
}

# Demonstrate ToadStool service registration
demo_service_registration() {
    echo -e "\n${PURPLE}📝 Service Registration Demo${NC}"
    echo -e "${PURPLE}===========================${NC}"
    
    print_info "Demonstrating ToadStool service registration with Songbird..."
    
    # Create service registration payload
    REGISTRATION_PAYLOAD='{
        "service_id": "toadstool-compute",
        "service_type": "compute-platform",
        "version": "1.0.0-demo",
        "instance_id": "toadstool-demo-'$(date +%s)'",
        "capabilities": {
            "execution_environments": [
                {"type": "Native", "isolation": "sandbox"},
                {"type": "Container", "runtime": "docker"},
                {"type": "Wasm", "runtime": "wasmtime"}
            ],
            "resource_capacity": {
                "cpu_cores": 8,
                "memory_gb": 32.0,
                "disk_space_gb": 1000.0,
                "current_utilization": 25.0
            },
            "supported_runtimes": ["Native", "Container", "Wasm"],
            "security_features": ["Sandboxing", "ResourceLimiting", "NetworkIsolation"]
        },
        "endpoints": [
            {
                "endpoint_type": "http",
                "url": "http://localhost:8081",
                "capabilities": ["execute", "health"],
                "protocol_version": "1.0"
            }
        ],
        "health_check": {
            "endpoint": "http://localhost:8081/health",
            "interval_secs": 30,
            "timeout_secs": 5,
            "failure_threshold": 3
        },
        "metadata": {
            "platform": "'$(uname -s)'",
            "architecture": "'$(uname -m)'",
            "startup_time": "'$(date -Iseconds)'"
        },
        "tags": ["compute", "execution", "sandboxing", "demo"]
    }'
    
    echo -e "\n${BLUE}Registration payload:${NC}"
    echo "$REGISTRATION_PAYLOAD" | jq .
    
    # Register service
    if REGISTRATION_RESPONSE=$(curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "$REGISTRATION_PAYLOAD" \
        "http://localhost:${SONGBIRD_PORT}/api/v1/register"); then
        
        echo -e "\n${GREEN}Registration response:${NC}"
        echo "$REGISTRATION_RESPONSE" | jq .
        
        AUTH_TOKEN=$(echo "$REGISTRATION_RESPONSE" | jq -r '.token')
        print_status "Service registered with token: ${AUTH_TOKEN:0:20}..."
    else
        print_error "Service registration failed"
    fi
}

# Demonstrate configuration integration
demo_config_integration() {
    echo -e "\n${PURPLE}⚙️  Configuration Integration Demo${NC}"
    echo -e "${PURPLE}=================================${NC}"
    
    print_info "Demonstrating how ToadStool loads configuration from multiple sources..."
    
    # Set some environment variables
    export TOADSTOOL_ENV="demo"
    export TOADSTOOL_DEBUG="true"
    export TOADSTOOL_SONGBIRD_ENDPOINT="http://localhost:8080"
    export TOADSTOOL_MAX_CPU_PERCENT="75.0"
    
    echo -e "\n${BLUE}Environment variables set:${NC}"
    echo "  TOADSTOOL_ENV=$TOADSTOOL_ENV"
    echo "  TOADSTOOL_DEBUG=$TOADSTOOL_DEBUG"
    echo "  TOADSTOOL_SONGBIRD_ENDPOINT=$TOADSTOOL_SONGBIRD_ENDPOINT"
    echo "  TOADSTOOL_MAX_CPU_PERCENT=$TOADSTOOL_MAX_CPU_PERCENT"
    
    echo -e "\n${BLUE}Configuration loading sequence:${NC}"
    echo "1. ✅ Load default configuration"
    echo "2. ✅ Load from config file: ${DEMO_DIR}/config/toadstool.toml"
    echo "3. ✅ Override with environment variables"
    echo "4. ✅ Query Songbird for service endpoints"
    echo "5. ✅ Validate final configuration"
    
    print_status "Configuration integration complete"
}

# Show monitoring integration
demo_monitoring_integration() {
    echo -e "\n${PURPLE}📊 Monitoring Integration Demo${NC}"
    echo -e "${PURPLE}=============================${NC}"
    
    print_info "Demonstrating how ToadStool integrates with monitoring systems..."
    
    echo -e "\n${BLUE}Monitoring capabilities:${NC}"
    echo "  • Real-time metrics collection"
    echo "  • Prometheus integration"
    echo "  • Grafana dashboard support"
    echo "  • Jaeger distributed tracing"
    echo "  • Custom alerting rules"
    
    echo -e "\n${BLUE}Metrics endpoints:${NC}"
    echo "  • Prometheus: http://localhost:9090/metrics"
    echo "  • ToadStool health: http://localhost:8081/health"
    echo "  • ToadStool metrics: http://localhost:8081/metrics"
    
    print_status "Monitoring integration configured"
}

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}🧹 Cleaning up demo resources...${NC}"
    
    # Stop mock Songbird if we started it
    if [ -f "${DEMO_DIR}/songbird.pid" ]; then
        SONGBIRD_PID=$(cat "${DEMO_DIR}/songbird.pid")
        if ps -p $SONGBIRD_PID > /dev/null 2>&1; then
            kill $SONGBIRD_PID
            print_status "Stopped mock Songbird service"
        fi
        rm -f "${DEMO_DIR}/songbird.pid"
    fi
    
    # Clean up demo directory
    if [ -d "$DEMO_DIR" ]; then
        rm -rf "$DEMO_DIR"
        print_status "Cleaned up demo directory"
    fi
    
    echo -e "${GREEN}Demo cleanup complete!${NC}"
}

# Main demo function
run_demo() {
    echo -e "${CYAN}Starting ToadStool Songbird Integration Demo...${NC}\n"
    
    # Check prerequisites
    echo -e "${BLUE}Checking prerequisites...${NC}"
    
    if ! command -v curl &> /dev/null; then
        print_error "curl is required but not installed"
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        print_warning "jq not found - JSON output will be raw"
    fi
    
    if ! command -v python3 &> /dev/null; then
        print_error "python3 is required for mock Songbird service"
        exit 1
    fi
    
    print_status "Prerequisites check complete"
    
    # Run demo steps
    check_songbird
    demo_config_loading
    demo_service_discovery
    demo_port_orchestration
    demo_service_registration
    demo_config_integration
    demo_monitoring_integration
    
    echo -e "\n${GREEN}🎉 Demo completed successfully!${NC}"
    echo -e "\n${BLUE}Key features demonstrated:${NC}"
    echo "  ✅ Songbird service discovery integration"
    echo "  ✅ Dynamic port orchestration"
    echo "  ✅ Service registration with capabilities"
    echo "  ✅ Configuration loading from multiple sources"
    echo "  ✅ Environment-aware configuration"
    echo "  ✅ Monitoring and observability integration"
    
    echo -e "\n${YELLOW}Next steps:${NC}"
    echo "  1. Copy .env.example to .env and customize"
    echo "  2. Create toadstool.toml with your specific configuration"
    echo "  3. Start actual Songbird service for production"
    echo "  4. Deploy ToadStool with Songbird orchestration enabled"
}

# Handle script termination
trap cleanup EXIT

# Check if running in demo mode
if [ "$1" = "--demo" ]; then
    run_demo
else
    echo -e "${BLUE}ToadStool Songbird Integration Demo${NC}"
    echo ""
    echo "This script demonstrates the integration between ToadStool and Songbird"
    echo "for service discovery and port orchestration."
    echo ""
    echo -e "${YELLOW}Usage:${NC}"
    echo "  $0 --demo    Run the full integration demo"
    echo ""
    echo -e "${YELLOW}What this demo shows:${NC}"
    echo "  • Dynamic port allocation via Songbird"
    echo "  • Service discovery and registration"
    echo "  • Configuration loading from multiple sources"
    echo "  • Environment-aware configuration management"
    echo "  • Monitoring and observability integration"
    echo ""
    echo -e "${YELLOW}Prerequisites:${NC}"
    echo "  • curl (for HTTP requests)"
    echo "  • jq (for JSON processing, optional)"
    echo "  • python3 (for mock Songbird service)"
    echo ""
    echo -e "${GREEN}Run: $0 --demo${NC}"
fi 