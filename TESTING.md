# RustDrop Testing Infrastructure

This document provides comprehensive information about the RustDrop testing infrastructure, including various test categories, execution methods, and best practices.

## 🎯 Overview

RustDrop includes a robust, multi-layered testing strategy designed to ensure high code quality, security, performance, and reliability. The testing infrastructure validates everything from individual functions to complete user workflows and production deployment scenarios.

## 📋 Test Categories

### 1. Unit Tests (`src/**/*`)

**Purpose**: Test individual functions and methods in isolation.

**Location**: Embedded within source files using `#[cfg(test)]`

**Coverage**:

- Configuration management and validation
- File operations and metadata extraction
- Network utilities and port management
- Device information handling
- API handler logic

**Run Command**:

```bash
cargo test --lib --bins
```

**Key Features**:

- Fast execution (< 30 seconds)
- High code coverage (>90% target)
- Isolated testing with mocks
- Property-based testing for edge cases

### 2. Integration Tests (`tests/integration_tests.rs`)

**Purpose**: Test component interactions and API endpoints.

**Coverage**:

- Complete HTTP request/response cycles
- File upload/download workflows
- API endpoint validation
- CORS functionality
- Error handling and edge cases

**Run Command**:

```bash
cargo test --test integration_tests
```

**Key Features**:

- Full Axum application testing
- Real HTTP server interactions
- Comprehensive API validation
- Multi-threaded operation testing

### 3. Security Tests (`tests/security_tests.rs`)

**Purpose**: Validate security measures and input sanitization.

**Coverage**:

- Path traversal attack prevention
- Input validation and sanitization
- CORS policy enforcement
- Rate limiting validation
- XSS and injection attack prevention

**Run Command**:

```bash
cargo test --test security_tests
```

**Key Features**:

- Malicious input simulation
- Attack vector validation
- Security policy enforcement
- Vulnerability assessment

### 4. End-to-End Tests (`tests/e2e_tests.rs`)

**Purpose**: Test complete user workflows and CLI integration.

**Coverage**:

- CLI argument processing
- Complete server lifecycle
- File operations workflow
- Concurrent user scenarios
- Real-world usage patterns

**Run Command**:

```bash
cargo test --test e2e_tests
```

**Key Features**:

- Real binary execution
- Complete workflow validation
- User experience testing
- Cross-platform compatibility

### 5. Stress Tests (`tests/stress_tests.rs`)

**Purpose**: Validate system behavior under extreme conditions.

**Coverage**:

- High-concurrency file operations
- Large directory handling (5000+ files)
- Memory usage validation
- Performance degradation testing
- Resource cleanup verification

**Run Command**:

```bash
cargo test --test stress_tests --release
```

**Key Features**:

- Multi-threaded stress testing
- Resource usage monitoring
- Performance baseline validation
- Edge case scenario testing

### 6. Property-Based Tests (`tests/property_tests.rs`)

**Purpose**: Validate properties that should hold for all inputs.

**Coverage**:

- UUID determinism across inputs
- File size accuracy validation
- Configuration serialization consistency
- Path handling correctness

**Run Command**:

```bash
cargo test --test property_tests
```

**Key Features**:

- Automated input generation
- Property validation
- Edge case discovery
- Regression testing

### 7. Performance Benchmarks (`benches/performance.rs`)

**Purpose**: Measure and track performance characteristics.

**Coverage**:

- File operation performance
- API response times
- Memory usage patterns
- Concurrent operation throughput

**Run Command**:

```bash
cargo bench --bench performance
```

**Key Features**:

- Statistical performance analysis
- Regression detection
- Throughput measurement
- Memory usage profiling

## 🚀 Quick Start

### Run All Tests (Recommended)

```bash
./scripts/run_tests.sh
```

### Quick Test Run (Essential Tests Only)

```bash
./scripts/run_tests.sh --quick
```

### Unit Tests Only

```bash
./scripts/run_tests.sh --unit-only
```

### Manual Test Execution

```bash
# Unit tests
cargo test --lib --bins

# Integration tests
cargo test --test integration_tests

# Security tests
cargo test --test security_tests

# End-to-end tests
cargo test --test e2e_tests

# Stress tests (use release mode for better performance)
cargo test --test stress_tests --release

# Property-based tests
cargo test --test property_tests

# Performance benchmarks
cargo bench --bench performance
```

## 🔧 CI/CD Integration

### GitHub Actions Pipeline

The project includes a comprehensive CI/CD pipeline with the following stages:

1. **Test Suite** - Core functionality validation
2. **Security Audit** - Vulnerability and compliance scanning
3. **Mutation Testing** - Test quality validation
4. **Performance Testing** - Load and regression testing
5. **Chaos Testing** - Resilience under adverse conditions
6. **Code Coverage** - Test coverage analysis
7. **Multi-platform Build** - Cross-platform compatibility
8. **Docker Security** - Container security validation
9. **Deployment Validation** - Production readiness testing

### Advanced CI/CD Features

- **Mutation Testing**: Uses `cargo-mutants` to validate test quality
- **Chaos Testing**: Validates behavior under CPU/memory stress and network issues
- **Security Scanning**: Automated vulnerability detection with Trivy and OWASP ZAP
- **Performance Regression**: Automated detection of performance degradation
- **Multi-architecture Builds**: ARM64 and AMD64 container support

## 📊 Test Configuration

### Configuration File (`tests/test_config.toml`)

Centralized test configuration allows easy adjustment of:

- Timeout values
- Concurrency settings
- Test data sizes
- Performance thresholds
- Resource limits

### Environment Variables

Key environment variables for test execution:

- `RUST_LOG`: Set log levels during testing
- `RUSTDROP_TEST_PORT`: Override default test port
- `RUSTDROP_TEST_DIR`: Override test directory location

## 🛠️ Development Workflow

### Pre-commit Testing

```bash
# Quick validation before committing
./scripts/run_tests.sh --quick
```

### Full Test Suite

```bash
# Complete testing before major changes
./scripts/run_tests.sh
```

### Performance Monitoring

```bash
# Regular performance validation
cargo bench --bench performance
```

### Security Validation

```bash
# Security-focused testing
cargo test --test security_tests
cargo audit
```

## 📈 Test Metrics and Reporting

### Coverage Reporting

- **Tool**: `cargo-llvm-cov`
- **Target**: >90% line coverage
- **Reports**: HTML format in `test-results/coverage/`

### Performance Metrics

- **Tool**: Criterion.rs
- **Metrics**: Latency, throughput, memory usage
- **Reports**: HTML format in `target/criterion/`

### Test Results

- **Location**: `test-results/` directory
- **Format**: Logs, HTML reports, JSON metrics
- **Retention**: Cleaned on each test run

## 🔍 Debugging Failed Tests

### View Test Logs

```bash
# Check specific test output
cat test-results/unit_results.log
cat test-results/integration_results.log
```

### Run Individual Tests

```bash
# Run specific test with output
cargo test test_name -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Docker Test Debugging

```bash
# Check container logs
docker logs rustdrop-test

# Interactive container access
docker run -it rustdrop:test /bin/sh
```

## ⚡ Performance Optimization

### Test Execution Speed

- Use `--release` flag for stress tests
- Enable parallel test execution
- Cache dependencies in CI/CD
- Use incremental compilation

### Resource Management

- Automatic cleanup of test artifacts
- Proper container lifecycle management
- Memory usage monitoring
- Timeout protection

## 🛡️ Security Testing

### Attack Vector Coverage

- Path traversal attempts
- Malicious input injection
- CORS policy validation
- Rate limiting verification

### Compliance Validation

- Container security standards
- OWASP security guidelines
- Dependency vulnerability scanning
- License compliance checking

## 📝 Test Data Management

### Test File Generation

- Automated test data creation
- Various file sizes and types
- Edge case filename patterns
- Cleanup automation

### Temporary Resources

- Isolated test environments
- Automatic cleanup procedures
- No persistent side effects
- Resource leak detection

## 🔮 Future Enhancements

### Planned Improvements

- [ ] Visual regression testing for UI components
- [ ] Fuzz testing integration
- [ ] Database integration testing
- [ ] Mobile platform testing
- [ ] Load testing automation
- [ ] A/B testing framework

### Continuous Improvement

- Regular test infrastructure updates
- Performance baseline adjustments
- Security test enhancement
- Coverage target increases

## 🤝 Contributing to Tests

### Adding New Tests

1. Choose appropriate test category
2. Follow existing patterns and conventions
3. Include comprehensive documentation
4. Validate with the full test suite
5. Update this documentation

### Test Quality Guidelines

- Write clear, descriptive test names
- Include both positive and negative test cases
- Test edge cases and error conditions
- Maintain fast execution times
- Ensure test isolation and repeatability

### Review Checklist

- [ ] Tests pass in isolation
- [ ] Tests pass in the full suite
- [ ] Performance impact is acceptable
- [ ] Security implications considered
- [ ] Documentation is updated
- [ ] CI/CD integration works correctly

---

**Note**: This testing infrastructure is designed to grow with the project. Regular reviews and updates ensure it continues to provide value and catch issues early in the development process.
