# CI/CD Pipeline Documentation

This document describes the comprehensive GitHub Actions CI/CD automation system for the Loka Stratum Bitcoin Mining Proxy project.

## Overview

The CI/CD system provides automated testing, building, security scanning, and deployment for the Loka Stratum project. It consists of three main workflows:

1. **Main CI/CD Pipeline** (`ci-cd.yml`) - Comprehensive testing and deployment
2. **Docker Pipeline** (`docker.yml`) - Container builds and security
3. **Database Migration Pipeline** (`migrate.yml`) - Schema validation and migrations

## Workflows

### 1. Main CI/CD Pipeline (`ci-cd.yml`)

**Triggers:**
- Push to `master`, `main`, or `develop` branches
- Pull requests to `master`, `main`, or `develop` branches  
- Release tags (`v*`)
- Nightly schedule (2 AM UTC)

**Jobs:**

#### Code Quality & Linting
- **Purpose**: Ensures code follows Rust standards and project conventions
- **Actions**: 
  - Code formatting check (`cargo fmt`)
  - Clippy linting with strict warnings
  - Dependency outdated check
- **Duration**: ~2-3 minutes
- **Parallel**: Yes

#### Security Scanning
- **Purpose**: Identifies security vulnerabilities and code quality issues
- **Actions**:
  - Cargo audit for dependency vulnerabilities
  - CodeQL static analysis for Rust
  - Security pattern detection
- **Duration**: ~3-5 minutes  
- **Parallel**: Yes

#### Tests (Matrix: Rust stable & 1.88.0)
- **Purpose**: Validates functionality across Rust versions
- **Services**: PostgreSQL 15 for integration testing
- **Actions**:
  - Unit tests (`cargo test --lib`)
  - Integration tests (`cargo test --test '*'`)
  - Documentation tests (`cargo test --doc`)
  - Database migration testing
- **Duration**: ~5-8 minutes per matrix job
- **Parallel**: Yes (per Rust version)

#### Performance Benchmarks
- **Purpose**: Validates performance requirements and detects regressions
- **Trigger**: Non-PR pushes or PRs with `benchmark` label
- **Actions**:
  - Critical path benchmarks
  - Metrics performance validation
  - Benchmark result archival
- **Duration**: ~3-5 minutes
- **Parallel**: Yes

#### Docker Build & Test
- **Purpose**: Builds and validates Docker images
- **Dependencies**: Code quality, security, tests must pass
- **Actions**:
  - Multi-platform builds (linux/amd64, linux/arm64)
  - GitHub Container Registry and Docker Hub publishing
  - Functional testing of built images
  - Health check validation
- **Duration**: ~8-12 minutes
- **Parallel**: Platform builds are parallel

#### Release Management
- **Purpose**: Automates release creation and artifact publishing
- **Trigger**: Version tags (`v*`)
- **Actions**:
  - Release artifact compilation
  - Changelog generation from git history
  - GitHub release creation with binaries
  - Docker image tagging and publishing
- **Duration**: ~5-8 minutes
- **Parallel**: No (sequential after other jobs)

#### Environment Deployments
- **Staging**: Automatic deployment on `develop` branch
- **Production**: Automatic deployment on stable release tags
- **Actions**:
  - Environment-specific configuration
  - Health check validation
  - Deployment verification
- **Duration**: ~3-5 minutes per environment
- **Parallel**: No (environment-specific)

#### Performance Regression Check (PR only)
- **Purpose**: Compares performance between PR and base branch
- **Actions**:
  - Benchmark current branch
  - Benchmark base branch  
  - Performance comparison reporting
- **Duration**: ~5-10 minutes
- **Parallel**: No (requires sequential benchmarking)

#### End-to-End Testing
- **Purpose**: Validates complete system functionality
- **Services**: PostgreSQL, monitoring stack
- **Actions**:
  - Full stack deployment
  - Health endpoint testing
  - Monitoring integration validation
  - Service interaction testing
- **Duration**: ~8-12 minutes
- **Parallel**: No (requires full stack)

### 2. Docker Pipeline (`docker.yml`)

**Triggers:**
- Push/PR with Docker-related file changes
- Weekly schedule (Monday 4 AM UTC) for security updates

**Jobs:**

#### Multi-variant Docker Builds
- **Variants**:
  - `release`: Production-optimized build
  - `debug`: Development build with debug symbols
  - `slim`: Minimal size build for resource-constrained environments
- **Platforms**: linux/amd64, linux/arm64 (where applicable)
- **Actions**:
  - Parallel builds with BuildKit caching
  - Functional testing per variant
  - Registry publishing (non-PR)
- **Duration**: ~6-10 minutes per variant

#### Container Security Scanning
- **Tools**:
  - Trivy for vulnerability scanning
  - Snyk for container security (if token provided)
  - SARIF report upload to GitHub Security
- **Actions**:
  - Image vulnerability assessment
  - Base image security validation
  - Security report generation
- **Duration**: ~3-5 minutes

#### Image Analysis
- **Purpose**: Optimizes image size and validates contents
- **Actions**:
  - Size and layer analysis
  - SBOM (Software Bill of Materials) generation
  - Image content validation
- **Duration**: ~2-3 minutes

#### Multi-architecture Testing
- **Purpose**: Ensures compatibility across platforms
- **Platforms**: linux/amd64, linux/arm64
- **Actions**:
  - Cross-platform compilation testing
  - Architecture-specific functionality validation
- **Duration**: ~5-8 minutes per platform

#### Resource & Performance Testing
- **Purpose**: Validates resource consumption and performance
- **Actions**:
  - Resource limit testing (512MB memory, 1 CPU)
  - Memory leak detection over time
  - Performance under constraints
- **Duration**: ~8-12 minutes

#### Dockerfile Linting
- **Tools**: hadolint for Dockerfile best practices
- **Actions**:
  - Security best practices validation
  - Dockerfile optimization suggestions
  - SARIF report generation
- **Duration**: ~1-2 minutes

### 3. Database Migration Pipeline (`migrate.yml`)

**Triggers:**
- Push/PR with migration or model changes
- Database schema modifications

**Jobs:**

#### Migration Testing (Matrix: PostgreSQL 13, 14, 15, 16)
- **Purpose**: Validates migrations across PostgreSQL versions
- **Actions**:
  - Fresh migration testing
  - Schema state verification
  - SeaORM entity generation and validation
  - Migration rollback testing
- **Duration**: ~5-8 minutes per PostgreSQL version

#### Schema Compatibility Testing
- **Purpose**: Ensures model-database schema compatibility
- **Actions**:
  - Model compilation testing
  - Foreign key constraint validation
  - Data integrity constraint testing
- **Duration**: ~3-5 minutes

#### Migration Performance Testing
- **Purpose**: Validates migration performance and scalability
- **Actions**:
  - Migration timing validation (<60 seconds)
  - Large dataset migration testing
  - Query performance validation
- **Duration**: ~5-10 minutes

## Required Secrets and Variables

### Repository Secrets

```bash
# Docker Registry Authentication
DOCKERHUB_USERNAME        # Docker Hub username
DOCKERHUB_TOKEN          # Docker Hub access token

# Security Scanning (Optional)
SNYK_TOKEN               # Snyk API token for container scanning

# Environment-specific (Optional)
STAGING_DEPLOY_KEY       # SSH key for staging deployment
PRODUCTION_DEPLOY_KEY    # SSH key for production deployment

# Database (For production deployments)
POSTGRES_PASSWORD        # PostgreSQL password for production
GRAFANA_PASSWORD        # Grafana admin password for production
```

### Environment Variables

```bash
# Development
DATABASE_URL=postgres://root:root@localhost:5432/loka_dev
RUST_LOG=debug
RUST_BACKTRACE=1

# Production
DATABASE_URL=postgres://loka:${POSTGRES_PASSWORD}@postgres:5432/loka
RUST_LOG=info
RUST_BACKTRACE=0
```

## Workflow Execution

### Parallel Execution Strategy

The CI/CD system maximizes efficiency through intelligent parallelization:

```mermaid
graph TB
    A[Code Push] --> B[Lint]
    A --> C[Security Scan]
    A --> D[Tests - Rust Stable]
    A --> E[Tests - Rust 1.88.0]
    
    B --> F[Docker Build]
    C --> F
    D --> F
    E --> F
    
    F --> G[Benchmarks]
    F --> H[E2E Tests]
    G --> I[Release/Deploy]
    H --> I
```

### Performance Optimization

**Caching Strategy:**
- Rust dependencies cached by `Cargo.lock` hash
- Docker layer caching with GitHub Actions cache
- Scope-specific caches for different job types

**Build Optimization:**
- Dependency pre-compilation in Docker
- Parallel test execution
- Matrix builds for multiple targets

**Resource Management:**
- PostgreSQL health checks before testing
- Service startup ordering with dependencies
- Cleanup on job completion

## Error Handling and Recovery

### Failure Scenarios

1. **Code Quality Failures**
   - **Symptom**: Formatting or linting errors
   - **Resolution**: Fix formatting/linting issues locally
   - **Prevention**: Use pre-commit hooks and IDE integration

2. **Test Failures**
   - **Symptom**: Unit/integration test failures
   - **Resolution**: Debug tests locally with same environment
   - **Prevention**: Run `make ci` before pushing

3. **Docker Build Failures**
   - **Symptom**: Docker build errors or functionality tests fail
   - **Resolution**: Test Docker builds locally with `make docker-test`
   - **Prevention**: Validate Dockerfile changes with hadolint

4. **Performance Regression**
   - **Symptom**: Benchmark failures or performance degradation
   - **Resolution**: Investigate changes causing regression
   - **Prevention**: Run performance tests locally with `make perf-test`

5. **Migration Failures**
   - **Symptom**: Database migration or schema validation errors
   - **Resolution**: Test migrations locally with test database
   - **Prevention**: Validate migrations with `make migrate` locally

### Recovery Procedures

#### Failed Deployment Recovery
```bash
# 1. Check deployment status
make status

# 2. Check logs for errors
make logs

# 3. Rollback if necessary (production)
docker-compose down
docker-compose up -d --scale stratum=0  # Stop application
# Fix issues
docker-compose up -d  # Restart with fixes
```

#### Database Migration Recovery
```bash
# 1. Check migration status
make migrate-status

# 2. Manual rollback if needed
cd migration
cargo run -- down

# 3. Fix migration issues
# Edit migration files

# 4. Re-run migrations
cargo run
```

## Monitoring and Observability

### CI/CD Metrics

**Build Performance:**
- Job execution times
- Cache hit rates
- Parallel execution efficiency
- Resource utilization

**Quality Metrics:**
- Test coverage trends
- Security vulnerability counts
- Performance benchmark trends
- Code quality scores

### Alerting

**Critical Alerts:**
- Production deployment failures
- Security vulnerability discoveries
- Performance regression above threshold
- Critical test failures

**Notification Channels:**
- GitHub Actions built-in notifications
- Optional Slack/Discord webhooks (configure in workflows)
- Email notifications for release managers

## Best Practices

### Development Workflow

1. **Before Pushing:**
   ```bash
   make quick-test    # Format, lint, test
   make docker-test   # Test Docker builds
   ```

2. **Before Merging PRs:**
   ```bash
   make ci           # Full CI simulation
   make perf-test    # Performance validation
   ```

3. **Before Releases:**
   ```bash
   make release-check  # Comprehensive checks
   ```

### Performance Considerations

**Critical Metrics to Monitor:**
- Counter operations: <10ns
- Gauge operations: <10ns  
- Histogram operations: <10ns
- Memory usage: <512MB baseline
- CPU usage: <50% under normal load

**Performance Regression Thresholds:**
- >20% degradation fails builds
- >10% degradation creates warnings
- Memory increase >100MB investigated

### Security Best Practices

**Image Security:**
- Multi-stage builds minimize attack surface
- Non-root user execution
- Minimal base images (Alpine Linux)
- Regular base image updates via Dependabot

**Dependency Security:**
- Weekly dependency audits
- Automated vulnerability scanning
- Grouping of related dependencies in Dependabot

**Secret Management:**
- Use GitHub secrets for sensitive data
- Environment-specific secret scoping
- Regular secret rotation recommendations

## Troubleshooting

### Common Issues

#### 1. Cache Issues
```bash
# Clear GitHub Actions caches
# Go to repository -> Actions -> Caches -> Delete relevant caches

# Clear local caches
make clean-all
```

#### 2. Docker Build Failures
```bash
# Test locally
make docker-build
make docker-test

# Check hadolint for Dockerfile issues
docker run --rm -i hadolint/hadolint < stratum/Dockerfile
```

#### 3. Migration Test Failures
```bash
# Test migrations locally
export DATABASE_URL=postgres://root:root@localhost:5432/test_db
make migrate
make test-integration
```

#### 4. Performance Test Failures
```bash
# Run benchmarks locally
make bench-critical
make validate-performance

# Compare with baseline
cd stratum
./run_benchmarks.sh compare
```

### Debug Mode

Enable debug mode in workflows by adding environment variables:

```yaml
env:
  RUST_LOG: debug
  RUST_BACKTRACE: full
  CI_DEBUG: true
```

### Manual Workflow Triggers

```bash
# Trigger workflows manually using GitHub CLI
gh workflow run ci-cd.yml
gh workflow run docker.yml
gh workflow run migrate.yml

# Trigger with specific inputs
gh workflow run ci-cd.yml --field enable_benchmark=true
```

## Customization

### Adding New Jobs

1. **Code Quality Jobs**: Add to `ci-cd.yml` parallel with existing lint/security
2. **Test Jobs**: Extend test matrix or add new test types
3. **Deployment Jobs**: Add environment-specific deployment steps

### Environment-specific Configuration

**Staging Environment:**
```yaml
deploy-staging:
  environment: staging
  steps:
    - name: Deploy to staging
      run: |
        # Add staging deployment logic
        kubectl apply -f k8s/staging/
```

**Production Environment:**
```yaml
deploy-production:
  environment: production
  needs: [docker-build, benchmark]
  if: startsWith(github.ref, 'refs/tags/v')
  steps:
    - name: Deploy to production
      run: |
        # Add production deployment logic
        kubectl apply -f k8s/production/
```

### Performance Tuning

**Reduce Build Times:**
- Increase cache usage
- Optimize dependency compilation
- Use smaller test matrices for PRs

**Parallel Optimization:**
- Group related tests
- Use job dependencies effectively  
- Balance resource usage across runners

## Metrics and Monitoring

### CI/CD Performance Metrics

Track these metrics in your monitoring dashboard:

```bash
# Build time trends
github_workflow_duration_seconds{workflow="CI/CD Pipeline"}

# Success rates
github_workflow_success_rate{workflow="CI/CD Pipeline"}

# Cache effectiveness
github_cache_hit_rate{cache_type="rust-dependencies"}
```

### Quality Gates

**Automatic Failure Conditions:**
- Any security vulnerability with severity >= HIGH
- Performance regression > 20%
- Test coverage decrease > 5%
- Docker image size increase > 50MB

**Warning Conditions:**
- Performance regression 10-20%
- Outdated dependencies > 30 days
- Docker image size increase 20-50MB

## Integration with External Services

### GitHub Integration

**Security:**
- SARIF upload for security findings
- Dependency graph integration
- Security advisory integration

**Project Management:**
- Automatic issue linking in PRs
- Label-based workflow triggers
- Milestone progress tracking

### Registry Integration

**GitHub Container Registry:**
- Automatic image publishing
- Multi-architecture support
- Package cleanup policies

**Docker Hub:**
- Official image distribution
- README synchronization
- Usage metrics collection

## Maintenance

### Weekly Tasks

- Review failed workflows and address issues
- Check performance benchmark trends
- Update documentation for new features
- Review and merge Dependabot PRs

### Monthly Tasks

- Analyze build performance metrics
- Review security scanning results
- Update CI/CD workflows for new requirements
- Test disaster recovery procedures

### Quarterly Tasks

- Review and update security policies
- Benchmark CI/CD performance against industry standards
- Plan infrastructure upgrades
- Conduct CI/CD system retrospectives

## Support and Escalation

### Getting Help

1. **Check Recent Issues**: Review GitHub Issues for similar problems
2. **Review Workflow Logs**: Examine failed job logs for error details
3. **Local Reproduction**: Use `make ci` to reproduce issues locally
4. **Documentation**: Consult this guide and workflow comments

### Escalation Process

1. **Level 1**: Self-service using documentation and logs
2. **Level 2**: Create GitHub Issue with workflow logs and error details
3. **Level 3**: Contact repository maintainers for complex CI/CD issues

### Contributing to CI/CD

1. **Test Changes**: Always test workflow changes in a fork first
2. **Documentation**: Update this document for any workflow changes
3. **Performance**: Consider impact on build times and resource usage
4. **Security**: Review security implications of any changes