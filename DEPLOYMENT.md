# Loka Stratum Deployment Guide

Complete production deployment guide for Loka Stratum Bitcoin Mining Proxy, covering all aspects of infrastructure setup, container orchestration, and production operations.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Environment Configuration](#environment-configuration)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Monitoring Setup](#monitoring-setup)
- [Network Configuration](#network-configuration)
- [Security Configuration](#security-configuration)
- [Database Setup](#database-setup)
- [Load Balancer Configuration](#load-balancer-configuration)
- [SSL/TLS Configuration](#ssltls-configuration)
- [Production Checklist](#production-checklist)

## Prerequisites

### System Requirements

**Minimum Requirements:**
- CPU: 2 cores, 2.4 GHz
- RAM: 4 GB
- Storage: 20 GB SSD
- Network: 100 Mbps

**Recommended Production:**
- CPU: 4+ cores, 3.0 GHz+
- RAM: 8+ GB
- Storage: 100+ GB SSD (NVMe preferred)
- Network: 1 Gbps+

### Dependencies

**Required Software:**
```bash
# Docker & Docker Compose
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo systemctl enable docker
sudo systemctl start docker

# Docker Compose (latest)
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Git
sudo apt-get update && sudo apt-get install -y git

# Optional: Build tools for custom builds
sudo apt-get install -y build-essential curl
```

**For Kubernetes Deployment:**
```bash
# kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

# Helm (optional)
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

## Quick Start

### 1. Clone and Setup

```bash
# Clone the repository
git clone https://github.com/loka/stratum.git
cd loka

# Make scripts executable
chmod +x stratum/docker-entrypoint.sh
chmod +x monitoring/deploy-sentry.sh
chmod +x monitoring/test-error-tracking.sh
```

### 2. Environment Configuration

```bash
# Copy environment template
cp monitoring/.env.example monitoring/.env
cp stratum/.env.example stratum/.env

# Edit configurations
nano monitoring/.env
nano stratum/.env
```

### 3. Quick Deploy

```bash
# Start monitoring stack first
cd monitoring
docker-compose up -d

# Initialize Sentry (wait 30 seconds after monitoring starts)
./deploy-sentry.sh

# Start Loka Stratum
cd ../stratum
docker-compose up -d

# Verify deployment
docker-compose ps
curl http://localhost:9090/health
```

## Environment Configuration

### Core Environment Variables

Create `stratum/.env`:
```bash
# Server Configuration
LOKA_SERVER_PORT=3333
LOKA_SERVER_BIND_ADDRESS=0.0.0.0

# Pool Configuration
LOKA_POOL_NAME=production_pool
LOKA_POOL_HOST=130.211.20.161
LOKA_POOL_PORT=9200
LOKA_POOL_USERNAME=your_btc_address_or_username
LOKA_POOL_PASSWORD=optional_password

# Database Configuration
DATABASE_URL=postgresql://loka_user:secure_password@postgres:5432/loka_stratum

# Security Configuration
LOKA_RATE_LIMIT_ENABLED=true
LOKA_MAX_CONNECTIONS=1000
LOKA_CONNECTION_TIMEOUT=30s

# Monitoring Configuration
LOKA_METRICS_ENABLED=true
LOKA_METRICS_PORT=9090
SENTRY_DSN=https://your-sentry-dsn@sentry:9000/1
SENTRY_ENVIRONMENT=production

# Logging Configuration
RUST_LOG=loka_stratum=info,tower_http=debug
LOKA_LOG_FORMAT=json
LOKA_LOG_FILE=/app/logs/loka-stratum.log
```

### Monitoring Environment Variables

Create `monitoring/.env`:
```bash
# Grafana Configuration
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=admin123

# Sentry Configuration
SENTRY_SECRET_KEY=your-long-random-secret-key-here
SENTRY_POSTGRES_PASSWORD=sentry_db_password
SENTRY_REDIS_PASSWORD=redis_password

# PostgreSQL Configuration
POSTGRES_USER=loka_user
POSTGRES_PASSWORD=secure_password
POSTGRES_DB=loka_stratum

# Prometheus Configuration
PROMETHEUS_RETENTION_TIME=30d
PROMETHEUS_RETENTION_SIZE=50GB
```

## Docker Deployment

### Single Host Deployment

**1. Production Docker Compose Configuration**

Create `docker-compose.production.yml`:
```yaml
version: '3.8'

services:
  loka-stratum:
    build:
      context: .
      dockerfile: stratum/Dockerfile
    ports:
      - "3333:3333"
      - "9090:9090"
    environment:
      - RUST_LOG=loka_stratum=info
      - DATABASE_URL=${DATABASE_URL}
      - SENTRY_DSN=${SENTRY_DSN}
    volumes:
      - ./config/loka-stratum.toml:/app/config/loka-stratum.toml:ro
      - ./logs:/app/logs
      - ./data:/app/data
    restart: unless-stopped
    networks:
      - loka-network
    depends_on:
      - postgres
      - redis
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '1.0'
          memory: 1G

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=${POSTGRES_USER}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
      - POSTGRES_DB=${POSTGRES_DB}
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./migration/init.sql:/docker-entrypoint-initdb.d/init.sql:ro
    restart: unless-stopped
    networks:
      - loka-network
    deploy:
      resources:
        limits:
          cpus: '1.0'
          memory: 1G

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis-data:/data
    restart: unless-stopped
    networks:
      - loka-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
      - ./nginx/logs:/var/log/nginx
    restart: unless-stopped
    networks:
      - loka-network
    depends_on:
      - loka-stratum

volumes:
  postgres-data:
  redis-data:

networks:
  loka-network:
    driver: bridge
```

**2. Deploy Production Stack**

```bash
# Create necessary directories
mkdir -p {config,logs,data,nginx/ssl,nginx/logs}

# Deploy with production configuration
docker-compose -f docker-compose.production.yml up -d

# Verify deployment
docker-compose -f docker-compose.production.yml ps
docker-compose -f docker-compose.production.yml logs -f loka-stratum
```

### Multi-Host Docker Swarm

**1. Initialize Docker Swarm**

```bash
# On manager node
docker swarm init --advertise-addr <MANAGER-IP>

# On worker nodes (use token from manager)
docker swarm join --token <TOKEN> <MANAGER-IP>:2377
```

**2. Deploy Stack**

Create `docker-stack.yml`:
```yaml
version: '3.8'

services:
  loka-stratum:
    image: loka-stratum:latest
    ports:
      - "3333:3333"
      - "9090:9090"
    environment:
      - RUST_LOG=loka_stratum=info
      - DATABASE_URL=postgresql://loka_user:secure_password@postgres:5432/loka_stratum
    deploy:
      replicas: 3
      update_config:
        parallelism: 1
        delay: 10s
      restart_policy:
        condition: on-failure
      placement:
        constraints:
          - node.role == worker
    networks:
      - loka-overlay

  postgres:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=loka_user
      - POSTGRES_PASSWORD=secure_password
      - POSTGRES_DB=loka_stratum
    volumes:
      - postgres-data:/var/lib/postgresql/data
    deploy:
      replicas: 1
      placement:
        constraints:
          - node.role == manager
    networks:
      - loka-overlay

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - nginx-config:/etc/nginx/conf.d
    deploy:
      replicas: 2
      update_config:
        parallelism: 1
        delay: 5s
    networks:
      - loka-overlay

volumes:
  postgres-data:
  nginx-config:

networks:
  loka-overlay:
    driver: overlay
    attachable: true
```

Deploy the stack:
```bash
docker stack deploy -c docker-stack.yml loka-production
docker stack services loka-production
docker stack ps loka-production
```

## Kubernetes Deployment

### 1. Namespace and ConfigMap

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: loka-stratum

---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: loka-stratum-config
  namespace: loka-stratum
data:
  loka-stratum.toml: |
    [server]
    port = 3333
    bind_address = "0.0.0.0"
    
    [pool]
    name = "production_pool"
    host = "130.211.20.161"
    port = 9200
    username = "your_username"
    
    [limiter]
    connections = 1000
    jobs = "10m"
    submissions = "2d"
    
    [metrics]
    enabled = true
    port = 9090
    prometheus_endpoint = "/metrics/prometheus"
```

### 2. Secrets

```yaml
# k8s/secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: loka-stratum-secrets
  namespace: loka-stratum
type: Opaque
data:
  database-url: <base64-encoded-database-url>
  sentry-dsn: <base64-encoded-sentry-dsn>
  pool-password: <base64-encoded-pool-password>
```

Create secrets:
```bash
kubectl create secret generic loka-stratum-secrets \
  --from-literal=database-url="postgresql://loka_user:secure_password@postgres:5432/loka_stratum" \
  --from-literal=sentry-dsn="https://your-sentry-dsn@sentry:9000/1" \
  --from-literal=pool-password="your_pool_password" \
  -n loka-stratum
```

### 3. Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: loka-stratum
  namespace: loka-stratum
  labels:
    app: loka-stratum
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  selector:
    matchLabels:
      app: loka-stratum
  template:
    metadata:
      labels:
        app: loka-stratum
    spec:
      containers:
      - name: loka-stratum
        image: loka-stratum:latest
        imagePullPolicy: Always
        ports:
        - name: stratum
          containerPort: 3333
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        env:
        - name: RUST_LOG
          value: "loka_stratum=info"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: loka-stratum-secrets
              key: database-url
        - name: SENTRY_DSN
          valueFrom:
            secretKeyRef:
              name: loka-stratum-secrets
              key: sentry-dsn
        - name: LOKA_POOL_PASSWORD
          valueFrom:
            secretKeyRef:
              name: loka-stratum-secrets
              key: pool-password
        resources:
          requests:
            cpu: 500m
            memory: 512Mi
          limits:
            cpu: 2000m
            memory: 2Gi
        volumeMounts:
        - name: config
          mountPath: /app/config
        - name: logs
          mountPath: /app/logs
        livenessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 9090
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: loka-stratum-config
      - name: logs
        emptyDir: {}
```

### 4. Service and Ingress

```yaml
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: loka-stratum-service
  namespace: loka-stratum
  labels:
    app: loka-stratum
spec:
  type: LoadBalancer
  ports:
  - name: stratum
    port: 3333
    targetPort: 3333
    protocol: TCP
  - name: metrics
    port: 9090
    targetPort: 9090
    protocol: TCP
  selector:
    app: loka-stratum

---
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: loka-stratum-ingress
  namespace: loka-stratum
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  tls:
  - hosts:
    - loka-stratum.example.com
    secretName: loka-stratum-tls
  rules:
  - host: loka-stratum.example.com
    http:
      paths:
      - path: /metrics
        pathType: Prefix
        backend:
          service:
            name: loka-stratum-service
            port:
              number: 9090
      - path: /health
        pathType: Prefix
        backend:
          service:
            name: loka-stratum-service
            port:
              number: 9090
```

### 5. Deploy to Kubernetes

```bash
# Apply all configurations
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/ingress.yaml

# Verify deployment
kubectl get all -n loka-stratum
kubectl logs -f deployment/loka-stratum -n loka-stratum

# Check service endpoints
kubectl get endpoints -n loka-stratum
kubectl get ingress -n loka-stratum
```

## Monitoring Setup

### 1. Deploy Monitoring Stack

```bash
# Deploy monitoring services
cd monitoring
docker-compose up -d

# Wait for services to start
sleep 30

# Initialize Sentry
./deploy-sentry.sh

# Verify monitoring stack
curl http://localhost:9090/targets  # Prometheus targets
curl http://localhost:3000          # Grafana
curl http://localhost:9000          # Sentry
```

### 2. Configure Prometheus for Kubernetes

```yaml
# k8s/prometheus-config.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: prometheus-config
  namespace: loka-stratum
data:
  prometheus.yml: |
    global:
      scrape_interval: 15s
      evaluation_interval: 15s
    
    rule_files:
      - "loka-stratum.yml"
    
    scrape_configs:
    - job_name: 'loka-stratum'
      kubernetes_sd_configs:
      - role: endpoints
        namespaces:
          names:
          - loka-stratum
      relabel_configs:
      - source_labels: [__meta_kubernetes_service_name]
        regex: loka-stratum-service
        action: keep
      - source_labels: [__meta_kubernetes_endpoint_port_name]
        regex: metrics
        action: keep
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: instance
      - source_labels: [__meta_kubernetes_namespace]
        target_label: kubernetes_namespace
      - source_labels: [__meta_kubernetes_service_name]
        target_label: kubernetes_name
```

## Network Configuration

### Firewall Rules

```bash
# UFW (Ubuntu)
sudo ufw allow 22/tcp     # SSH
sudo ufw allow 80/tcp     # HTTP
sudo ufw allow 443/tcp    # HTTPS
sudo ufw allow 3333/tcp   # Stratum
sudo ufw allow 9090/tcp   # Metrics (restrict to monitoring network)
sudo ufw enable

# iptables (RHEL/CentOS)
firewall-cmd --permanent --add-port=22/tcp
firewall-cmd --permanent --add-port=80/tcp
firewall-cmd --permanent --add-port=443/tcp
firewall-cmd --permanent --add-port=3333/tcp
firewall-cmd --permanent --add-port=9090/tcp
firewall-cmd --reload
```

### Load Balancer Configuration

**HAProxy Configuration:**
```
# /etc/haproxy/haproxy.cfg
global
    daemon
    log stdout local0
    maxconn 4096

defaults
    mode tcp
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms

# Stratum mining protocol
frontend stratum_frontend
    bind *:3333
    mode tcp
    default_backend stratum_servers

backend stratum_servers
    mode tcp
    balance roundrobin
    option tcp-check
    tcp-check connect port 3333
    server stratum1 10.0.1.10:3333 check
    server stratum2 10.0.1.11:3333 check
    server stratum3 10.0.1.12:3333 check

# HTTP metrics and health
frontend http_frontend
    bind *:80
    mode http
    default_backend http_servers

backend http_servers
    mode http
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200
    server web1 10.0.1.10:9090 check
    server web2 10.0.1.11:9090 check
    server web3 10.0.1.12:9090 check
```

**Nginx Load Balancer:**
```nginx
# /etc/nginx/nginx.conf
upstream stratum_backend {
    least_conn;
    server 10.0.1.10:3333 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:3333 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:3333 max_fails=3 fail_timeout=30s;
}

upstream metrics_backend {
    least_conn;
    server 10.0.1.10:9090 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:9090 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:9090 max_fails=3 fail_timeout=30s;
}

server {
    listen 3333;
    proxy_pass stratum_backend;
    proxy_timeout 60s;
    proxy_connect_timeout 5s;
}

server {
    listen 80;
    location /metrics {
        proxy_pass http://metrics_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    location /health {
        proxy_pass http://metrics_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Database Setup

### PostgreSQL Setup

**1. Install PostgreSQL**
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install postgresql postgresql-contrib

# RHEL/CentOS
sudo dnf install postgresql postgresql-server postgresql-contrib
sudo postgresql-setup --initdb
```

**2. Configure PostgreSQL**
```bash
# Edit postgresql.conf
sudo nano /etc/postgresql/15/main/postgresql.conf

# Key settings for production
listen_addresses = '*'
max_connections = 200
shared_buffers = 2GB
work_mem = 4MB
maintenance_work_mem = 512MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100

# Configure pg_hba.conf
sudo nano /etc/postgresql/15/main/pg_hba.conf

# Add connection rules
host    loka_stratum    loka_user    0.0.0.0/0    md5
```

**3. Create Database and User**
```sql
-- Connect as postgres user
sudo -u postgres psql

-- Create database and user
CREATE DATABASE loka_stratum;
CREATE USER loka_user WITH ENCRYPTED PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE loka_stratum TO loka_user;

-- Grant additional permissions
GRANT USAGE ON SCHEMA public TO loka_user;
GRANT CREATE ON SCHEMA public TO loka_user;
ALTER USER loka_user CREATEDB;

-- Exit psql
\q
```

**4. Run Migrations**
```bash
# From the project root directory
cd migration
DATABASE_URL="postgresql://loka_user:secure_password@localhost:5432/loka_stratum" cargo run
```

### Database Performance Tuning

**PostgreSQL Optimizations:**
```sql
-- Enable connection pooling
CREATE EXTENSION IF NOT EXISTS pgbouncer;

-- Optimize for mining workloads
ALTER SYSTEM SET random_page_cost = 1.1;
ALTER SYSTEM SET effective_cache_size = '6GB';
ALTER SYSTEM SET shared_preload_libraries = 'pg_stat_statements';

-- Create indexes for common queries
CREATE INDEX CONCURRENTLY idx_workers_miner_id ON workers(miner_id);
CREATE INDEX CONCURRENTLY idx_submissions_timestamp ON submissions(timestamp);
CREATE INDEX CONCURRENTLY idx_submissions_worker_id ON submissions(worker_id);
```

## SSL/TLS Configuration

### Let's Encrypt with Certbot

```bash
# Install certbot
sudo apt-get install certbot python3-certbot-nginx

# Generate certificate
sudo certbot --nginx -d loka-stratum.example.com

# Auto-renewal cron job
echo "0 12 * * * /usr/bin/certbot renew --quiet" | sudo crontab -
```

### Custom SSL Certificate

```bash
# Generate private key
openssl genrsa -out loka-stratum.key 2048

# Generate certificate signing request
openssl req -new -key loka-stratum.key -out loka-stratum.csr

# Generate self-signed certificate (for testing)
openssl x509 -req -days 365 -in loka-stratum.csr -signkey loka-stratum.key -out loka-stratum.crt

# Copy certificates to nginx
sudo cp loka-stratum.{key,crt} /etc/nginx/ssl/
sudo chown root:root /etc/nginx/ssl/loka-stratum.*
sudo chmod 600 /etc/nginx/ssl/loka-stratum.key
sudo chmod 644 /etc/nginx/ssl/loka-stratum.crt
```

### Nginx SSL Configuration

```nginx
server {
    listen 443 ssl http2;
    server_name loka-stratum.example.com;

    ssl_certificate /etc/nginx/ssl/loka-stratum.crt;
    ssl_certificate_key /etc/nginx/ssl/loka-stratum.key;
    
    # SSL settings
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    
    # HSTS
    add_header Strict-Transport-Security "max-age=31536000" always;
    
    location /metrics {
        proxy_pass http://localhost:9090;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    location /health {
        proxy_pass http://localhost:9090;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Production Checklist

### Pre-Deployment

- [ ] **System Requirements**: Verify CPU, RAM, storage, network capacity
- [ ] **Dependencies**: Install Docker, Docker Compose, required tools
- [ ] **Network**: Configure firewall rules and load balancer
- [ ] **SSL Certificates**: Obtain and install SSL certificates
- [ ] **Database**: Set up PostgreSQL with proper user and permissions
- [ ] **Monitoring**: Deploy monitoring stack (Prometheus, Grafana, Sentry)
- [ ] **Backup**: Configure automated backup procedures
- [ ] **Security**: Review security configuration and credentials

### Configuration Review

- [ ] **Environment Variables**: Set all required environment variables
- [ ] **Pool Configuration**: Verify mining pool connection details
- [ ] **Resource Limits**: Set appropriate CPU and memory limits
- [ ] **Connection Limits**: Configure max connections based on capacity
- [ ] **Logging**: Set appropriate log levels and rotation
- [ ] **Metrics**: Enable metrics collection and export
- [ ] **Health Checks**: Configure health check endpoints

### Deployment Verification

- [ ] **Service Health**: All services start successfully
- [ ] **Database Connection**: Database migrations run successfully
- [ ] **Pool Connection**: Stratum proxy connects to mining pool
- [ ] **Miner Connection**: Test miners can connect successfully
- [ ] **Metrics Collection**: Prometheus scraping metrics successfully
- [ ] **Alerting**: Alerts are configured and firing when appropriate
- [ ] **Performance**: System performance meets expectations

### Post-Deployment

- [ ] **Monitoring Dashboard**: Set up Grafana dashboards
- [ ] **Alert Rules**: Configure alert rules for critical metrics
- [ ] **Log Aggregation**: Set up log collection and analysis
- [ ] **Backup Verification**: Test backup and restore procedures
- [ ] **Documentation**: Update runbooks and documentation
- [ ] **Team Training**: Train operations team on new deployment
- [ ] **Runbook Testing**: Test incident response procedures

### Ongoing Maintenance

- [ ] **Security Updates**: Schedule regular security updates
- [ ] **Certificate Renewal**: Set up automatic SSL certificate renewal
- [ ] **Log Rotation**: Configure log rotation and cleanup
- [ ] **Database Maintenance**: Schedule database maintenance tasks
- [ ] **Performance Monitoring**: Monitor system performance trends
- [ ] **Capacity Planning**: Plan for capacity scaling
- [ ] **Disaster Recovery**: Test disaster recovery procedures

## Common Issues and Solutions

### Port Binding Issues
```bash
# Check which process is using port 3333
sudo netstat -tlnp | grep :3333
sudo lsof -i :3333

# Kill process using port
sudo kill -9 <PID>
```

### Permission Issues
```bash
# Fix Docker permissions
sudo usermod -aG docker $USER
newgrp docker

# Fix file permissions
sudo chown -R $USER:$USER logs data config
```

### Database Connection Issues
```bash
# Test database connection
psql -h localhost -U loka_user -d loka_stratum -c "SELECT 1;"

# Check database logs
sudo tail -f /var/log/postgresql/postgresql-15-main.log
```

### Memory Issues
```bash
# Monitor memory usage
docker stats
free -h
top -o %MEM

# Increase swap if needed
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

## Support and Troubleshooting

For additional support:
- Review the [Operations Manual](OPERATIONS.md)
- Check the [Monitoring Guide](MONITORING.md)
- Consult the [Security Guide](SECURITY.md)
- See the [API Documentation](API.md)

For production support:
- Email: support@loka-stratum.org
- Documentation: https://docs.loka-stratum.org
- GitHub Issues: https://github.com/loka/stratum/issues