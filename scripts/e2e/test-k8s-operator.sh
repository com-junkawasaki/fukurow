#!/bin/bash

# Fukurow Kubernetes Operator E2E Test Script
# This script tests the complete operator lifecycle in a Kubernetes environment

set -e

# Configuration
NAMESPACE="${NAMESPACE:-fukurow-test}"
CLUSTER_NAME="${CLUSTER_NAME:-test-cluster}"
TIMEOUT="${TIMEOUT:-600}"  # 10 minutes

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if ! command -v kubectl &> /dev/null; then
        log_error "kubectl is required but not installed"
        exit 1
    fi

    if ! command -v docker &> /dev/null; then
        log_error "docker is required but not installed"
        exit 1
    fi

    # Check if kubectl can connect to cluster
    if ! kubectl cluster-info &> /dev/null; then
        log_error "Cannot connect to Kubernetes cluster"
        exit 1
    fi

    log_info "Prerequisites check passed"
}

# Setup test environment
setup_test_env() {
    log_info "Setting up test environment..."

    # Create test namespace
    kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

    # Build and load operator image (if using kind/minikube)
    if kubectl config current-context | grep -q "kind\|minikube"; then
        log_info "Building operator image for local cluster..."

        # Build operator image
        docker build -t fukurow-operator:test -f Dockerfile.operator .

        # Load image into cluster
        if kubectl config current-context | grep -q "kind"; then
            kind load docker-image fukurow-operator:test --name "$(kubectl config current-context | sed 's/kind-//')"
        elif kubectl config current-context | grep -q "minikube"; then
            minikube image load fukurow-operator:test
        fi
    fi

    log_info "Test environment setup complete"
}

# Install operator
install_operator() {
    log_info "Installing Fukurow operator..."

    # Apply CRD
    kubectl apply -f dist/kubernetes/crd.yaml
    kubectl wait --for=condition=established --timeout=60s crd/fukurowclusters.fukurow.io

    # Apply RBAC
    sed "s/fukurow-system/$NAMESPACE/g" dist/kubernetes/operator-rbac.yaml | kubectl apply -f -

    # Apply operator deployment (with test image if local)
    if kubectl config current-context | grep -q "kind\|minikube"; then
        sed "s/:latest/:test/g" dist/kubernetes/operator-deployment.yaml | kubectl apply -n "$NAMESPACE" -f -
    else
        kubectl apply -n "$NAMESPACE" -f dist/kubernetes/operator-deployment.yaml
    fi

    # Wait for operator to be ready
    log_info "Waiting for operator to be ready..."
    kubectl wait --for=condition=available --timeout=300s deployment/fukurow-operator -n "$NAMESPACE"

    log_info "Operator installation complete"
}

# Test cluster creation
test_cluster_creation() {
    log_info "Testing cluster creation..."

    # Apply test cluster
    cat <<EOF | kubectl apply -f -
apiVersion: fukurow.io/v1
kind: FukurowCluster
metadata:
  name: $CLUSTER_NAME
  namespace: $NAMESPACE
spec:
  replicas: 1
  image:
    registry: ghcr.io/gftdcojp
    repository: fukurow
    tag: latest
    pullPolicy: IfNotPresent
  resources:
    requests:
      cpu: 100m
      memory: 128Mi
    limits:
      cpu: 200m
      memory: 256Mi
  config:
    server:
      port: 3000
      host: 0.0.0.0
      maxConnections: 100
      timeoutSeconds: 30
    engine:
      maxConcurrentTasks: 5
      reasoningTimeoutSeconds: 30
      batchSize: 50
    security:
      tlsEnabled: false
  storage:
    storageType: emptyDir
    size: 1Gi
  monitoring:
    prometheusEnabled: true
    metricsPort: 9090
    healthChecksEnabled: true
    healthPort: 8080
EOF

    # Wait for cluster to be ready
    log_info "Waiting for Fukurow cluster to be ready..."
    local start_time=$(date +%s)
    while true; do
        local status=$(kubectl get fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null || echo "Unknown")

        if [ "$status" = "Running" ]; then
            log_info "Fukurow cluster is running!"
            break
        fi

        local elapsed=$(( $(date +%s) - start_time ))
        if [ $elapsed -gt $TIMEOUT ]; then
            log_error "Timeout waiting for cluster to be ready. Current status: $status"
            kubectl describe fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE"
            exit 1
        fi

        log_info "Waiting for cluster... (status: $status, elapsed: ${elapsed}s)"
        sleep 10
    done
}

# Test cluster functionality
test_cluster_functionality() {
    log_info "Testing cluster functionality..."

    # Get service details
    local service_name="${CLUSTER_NAME}-service"
    local pod_name=$(kubectl get pods -n "$NAMESPACE" -l "app.kubernetes.io/instance=$CLUSTER_NAME" -o jsonpath='{.items[0].metadata.name}')

    # Wait for service to be ready
    kubectl wait --for=condition=ready --timeout=60s pod/"$pod_name" -n "$NAMESPACE"

    # Test health endpoint
    log_info "Testing health endpoint..."
    local health_url=$(kubectl get svc "$service_name" -n "$NAMESPACE" -o jsonpath='{.spec.clusterIP}'):3000/health

    # Port forward for testing (if using local cluster)
    if kubectl config current-context | grep -q "kind\|minikube"; then
        kubectl port-forward -n "$NAMESPACE" svc/"$service_name" 3000:3000 &
        local port_forward_pid=$!
        sleep 5

        # Test health endpoint
        if curl -f http://localhost:3000/health &>/dev/null; then
            log_info "Health check passed!"
        else
            log_error "Health check failed!"
            kill $port_forward_pid 2>/dev/null || true
            exit 1
        fi

        kill $port_forward_pid 2>/dev/null || true
    fi

    log_info "Cluster functionality test passed"
}

# Test scaling
test_scaling() {
    log_info "Testing cluster scaling..."

    # Scale up
    kubectl patch fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE" --type merge -p '{"spec":{"replicas": 2}}'

    # Wait for scaling
    log_info "Waiting for scale up..."
    local start_time=$(date +%s)
    while true; do
        local ready_replicas=$(kubectl get fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo "0")

        if [ "$ready_replicas" = "2" ]; then
            log_info "Scale up successful!"
            break
        fi

        local elapsed=$(( $(date +%s) - start_time ))
        if [ $elapsed -gt 120 ]; then
            log_error "Timeout waiting for scale up. Ready replicas: $ready_replicas"
            exit 1
        fi

        sleep 5
    done

    # Scale down
    kubectl patch fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE" --type merge -p '{"spec":{"replicas": 1}}'

    log_info "Waiting for scale down..."
    start_time=$(date +%s)
    while true; do
        local ready_replicas=$(kubectl get fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo "0")

        if [ "$ready_replicas" = "1" ]; then
            log_info "Scale down successful!"
            break
        fi

        local elapsed=$(( $(date +%s) - start_time ))
        if [ $elapsed -gt 120 ]; then
            log_error "Timeout waiting for scale down. Ready replicas: $ready_replicas"
            exit 1
        fi

        sleep 5
    done

    log_info "Scaling test passed"
}

# Cleanup
cleanup() {
    log_info "Cleaning up test resources..."

    # Delete cluster
    kubectl delete fukurowcluster "$CLUSTER_NAME" -n "$NAMESPACE" --ignore-not-found=true

    # Delete operator
    kubectl delete -n "$NAMESPACE" -f dist/kubernetes/operator-deployment.yaml --ignore-not-found=true
    kubectl delete -f dist/kubernetes/operator-rbac.yaml --ignore-not-found=true

    # Delete CRD
    kubectl delete -f dist/kubernetes/crd.yaml --ignore-not-found=true

    # Delete namespace
    kubectl delete namespace "$NAMESPACE" --ignore-not-found=true

    log_info "Cleanup complete"
}

# Main test execution
main() {
    log_info "Starting Fukurow Kubernetes Operator E2E tests..."

    trap cleanup EXIT

    check_prerequisites
    setup_test_env
    install_operator
    test_cluster_creation
    test_cluster_functionality
    test_scaling

    log_info "🎉 All E2E tests passed!"
}

# Run main function
main "$@"
