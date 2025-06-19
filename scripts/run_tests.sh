#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_RESULTS_DIR="$PROJECT_ROOT/test-results"
FAILED_TESTS=()

# Create test results directory
mkdir -p "$TEST_RESULTS_DIR"

# Logging function
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
    FAILED_TESTS+=("$1")
}

log_info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
}

# Function to run a test category with timeout and result capture
run_test_category() {
    local category="$1"
    local description="$2"
    local command="$3"
    local timeout="${4:-300}"  # Default 5 minutes
    
    log "Running $description..."
    echo -e "${PURPLE}═══════════════════════════════════════════════════════════════${NC}"
    
    local start_time=$(date +%s)
    local result_file="$TEST_RESULTS_DIR/${category}_results.log"
    
    if timeout "${timeout}s" bash -c "$command" > "$result_file" 2>&1; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_success "$description completed in ${duration}s"
        
        # Show summary if available
        if grep -q "test result:" "$result_file"; then
            grep "test result:" "$result_file" | tail -1
        fi
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            log_error "$description timed out after ${timeout}s"
        else
            log_error "$description failed with exit code $exit_code"
        fi
        
        # Show last few lines of output for debugging
        echo -e "${RED}Last 10 lines of output:${NC}"
        tail -10 "$result_file" || true
    fi
    
    echo ""
}

# Function to check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if cargo is installed
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo is not installed. Please install Rust."
        exit 1
    fi
    
    # Check if required tools are available
    local missing_tools=()
    
    if ! command -v docker &> /dev/null; then
        missing_tools+=("docker")
    fi
    
    if ! command -v curl &> /dev/null; then
        missing_tools+=("curl")
    fi
    
    if [ ${#missing_tools[@]} -gt 0 ]; then
        log_warning "Missing tools: ${missing_tools[*]}. Some tests may be skipped."
    fi
    
    log_success "Prerequisites check completed"
}

# Function to clean up previous test artifacts
cleanup() {
    log "Cleaning up previous test artifacts..."
    
    # Clean cargo build artifacts for tests
    cargo clean --package rustdrop --quiet || true
    
    # Remove any leftover test files
    find "$PROJECT_ROOT" -name "test_*" -type f -delete 2>/dev/null || true
    find "$PROJECT_ROOT" -name "*.tmp" -type f -delete 2>/dev/null || true
    
    # Kill any leftover rustdrop processes
    pkill -f "rustdrop" 2>/dev/null || true
    
    # Clean up any test Docker containers
    docker ps -a --filter "name=rustdrop-test" --format "{{.ID}}" | xargs docker rm -f 2>/dev/null || true
    
    log_success "Cleanup completed"
}

# Function to generate test report
generate_report() {
    local report_file="$TEST_RESULTS_DIR/test_summary.md"
    
    log "Generating test report..."
    
    cat > "$report_file" << EOF
# RustDrop Test Execution Report

**Generated on:** $(date)
**Duration:** $(($(date +%s) - SCRIPT_START_TIME)) seconds

## Test Categories Executed

EOF

    # Add results for each category
    for result_file in "$TEST_RESULTS_DIR"/*_results.log; do
        if [ -f "$result_file" ]; then
            local category=$(basename "$result_file" _results.log)
            echo "### $category" >> "$report_file"
            echo "" >> "$report_file"
            
            if grep -q "test result: ok" "$result_file"; then
                echo "✅ **Status:** PASSED" >> "$report_file"
            elif grep -q "test result: FAILED" "$result_file"; then
                echo "❌ **Status:** FAILED" >> "$report_file"
            else
                echo "⚠️ **Status:** UNKNOWN" >> "$report_file"
            fi
            
            # Add test summary if available
            if grep -q "test result:" "$result_file"; then
                echo "" >> "$report_file"
                echo "```" >> "$report_file"
                grep "test result:" "$result_file" | tail -1 >> "$report_file"
                echo "```" >> "$report_file"
            fi
            
            echo "" >> "$report_file"
        fi
    done
    
    # Add failed tests summary
    if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
        echo "## ❌ Failed Tests" >> "$report_file"
        echo "" >> "$report_file"
        for failed_test in "${FAILED_TESTS[@]}"; do
            echo "- $failed_test" >> "$report_file"
        done
        echo "" >> "$report_file"
    fi
    
    echo "## 📊 Coverage and Performance" >> "$report_file"
    echo "" >> "$report_file"
    echo "- Coverage reports available in: \`$TEST_RESULTS_DIR/coverage/\`" >> "$report_file"
    echo "- Performance benchmarks available in: \`$PROJECT_ROOT/target/criterion/\`" >> "$report_file"
    
    log_success "Test report generated: $report_file"
}

# Main execution
main() {
    SCRIPT_START_TIME=$(date +%s)
    
    echo -e "${PURPLE}"
    cat << "EOF"
██████╗ ██╗   ██╗███████╗████████╗██████╗ ██████╗  ██████╗ ██████╗ 
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██╔══██╗██╔═══██╗██╔══██╗
██████╔╝██║   ██║███████╗   ██║   ██║  ██║██████╔╝██║   ██║██████╔╝
██╔══██╗██║   ██║╚════██║   ██║   ██║  ██║██╔══██╗██║   ██║██╔═══╝ 
██║  ██║╚██████╔╝███████║   ██║   ██████╔╝██║  ██║╚██████╔╝██║     
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝     
                                                                    
████████╗███████╗███████╗████████╗    ███████╗██╗   ██╗██╗████████╗███████╗
╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝    ██╔════╝██║   ██║██║╚══██╔══╝██╔════╝
   ██║   █████╗  ███████╗   ██║       ███████╗██║   ██║██║   ██║   █████╗  
   ██║   ██╔══╝  ╚════██║   ██║       ╚════██║██║   ██║██║   ██║   ██╔══╝  
   ██║   ███████╗███████║   ██║       ███████║╚██████╔╝██║   ██║   ███████╗
   ╚═╝   ╚══════╝╚══════╝   ╚═╝       ╚══════╝ ╚═════╝ ╚═╝   ╚═╝   ╚══════╝
EOF
    echo -e "${NC}"
    
    log_info "Starting comprehensive test suite for RustDrop"
    log_info "Results will be saved to: $TEST_RESULTS_DIR"
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Parse command line arguments
    RUN_UNIT=true
    RUN_INTEGRATION=true
    RUN_E2E=true
    RUN_STRESS=true
    RUN_SECURITY=true
    RUN_PERFORMANCE=true
    RUN_COVERAGE=true
    QUICK_MODE=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --quick)
                QUICK_MODE=true
                RUN_STRESS=false
                RUN_PERFORMANCE=false
                shift
                ;;
            --unit-only)
                RUN_INTEGRATION=false
                RUN_E2E=false
                RUN_STRESS=false
                RUN_SECURITY=false
                RUN_PERFORMANCE=false
                shift
                ;;
            --no-coverage)
                RUN_COVERAGE=false
                shift
                ;;
            --help)
                echo "Usage: $0 [options]"
                echo "Options:"
                echo "  --quick       Run only essential tests (skip stress/performance)"
                echo "  --unit-only   Run only unit tests"
                echo "  --no-coverage Skip coverage reporting"
                echo "  --help        Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Initialize
    check_prerequisites
    cleanup
    
    # Run test categories
    if [ "$RUN_UNIT" = true ]; then
        run_test_category "unit" "Unit Tests" \
            "cargo test --lib --bins --verbose" 120
    fi
    
    if [ "$RUN_INTEGRATION" = true ]; then
        run_test_category "integration" "Integration Tests" \
            "cargo test --test integration_tests --verbose" 180
    fi
    
    if [ "$RUN_SECURITY" = true ]; then
        run_test_category "security" "Security Tests" \
            "cargo test --test security_tests --verbose" 240
    fi
    
    if [ "$RUN_E2E" = true ]; then
        run_test_category "e2e" "End-to-End Tests" \
            "cargo test --test e2e_tests --verbose" 300
    fi
    
    if [ "$RUN_STRESS" = true ]; then
        run_test_category "stress" "Stress Tests" \
            "cargo test --test stress_tests --verbose --release" 600
    fi
    
    if [ "$RUN_PERFORMANCE" = true ]; then
        run_test_category "benchmarks" "Performance Benchmarks" \
            "cargo bench --bench performance" 400
    fi
    
    if [ "$RUN_COVERAGE" = true ]; then
        run_test_category "coverage" "Code Coverage Analysis" \
            "cargo llvm-cov --all-features --workspace --html --output-dir $TEST_RESULTS_DIR/coverage" 300
    fi
    
    # Linting and formatting checks
    run_test_category "format" "Code Formatting Check" \
        "cargo fmt --all -- --check" 60
    
    run_test_category "clippy" "Linting (Clippy)" \
        "cargo clippy --all-targets --all-features -- -D warnings" 120
    
    # Generate final report
    generate_report
    
    # Final summary
    echo -e "${PURPLE}═══════════════════════════════════════════════════════════════${NC}"
    
    if [ ${#FAILED_TESTS[@]} -eq 0 ]; then
        log_success "All tests passed! 🎉"
        echo -e "${GREEN}The RustDrop test suite completed successfully.${NC}"
        exit 0
    else
        log_error "Some tests failed:"
        for failed_test in "${FAILED_TESTS[@]}"; do
            echo -e "${RED}  - $failed_test${NC}"
        done
        echo ""
        echo -e "${RED}Check the detailed logs in $TEST_RESULTS_DIR for more information.${NC}"
        exit 1
    fi
}

# Trap to ensure cleanup on script exit
trap cleanup EXIT

# Run main function with all arguments
main "$@" 