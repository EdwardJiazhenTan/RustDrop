# RustDrop Deployment Guide

## 🐳 Docker Deployment

### Quick Start with Docker

```bash
# Build the image
docker build -t rustdrop .

# Run with default settings
docker run -p 8080:8080 -v ./files:/app/files rustdrop

# Access at http://localhost:8080
```

### Using Docker Compose

```bash
# Start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down
```

### Configuration

#### Environment Variables

- `RUSTDROP_SERVER__PORT` - Server port (default: 8080)
- `RUSTDROP_SERVER__HOST` - Server host (default: 0.0.0.0)
- `RUSTDROP_SERVER__MAX_FILE_SIZE` - Max upload size in bytes (default: 1GB)
- `RUSTDROP_FILES__DIRECTORY` - Upload directory (default: /app/files)
- `RUSTDROP_UI__QR_CODE` - Enable QR codes (default: true)
- `RUSTDROP_DISCOVERY__ENABLED` - Enable mDNS discovery (default: true)

#### Configuration File

Create a `rustdrop.toml` file:

```toml
[server]
port = 8080
host = "0.0.0.0"
max_file_size = 1073741824  # 1GB

[files]
# directory = "/custom/path"  # Optional custom directory

[discovery]
enabled = true

[ui]
qr_code = true
open_browser = false
```

Generate an example config:

```bash
rustdrop --generate-config
```

## 🚀 GitHub Actions CI/CD

The repository includes automated CI/CD pipeline that:

1. **Tests** - Runs tests, formatting, and linting on every PR
2. **Build** - Creates multi-platform Docker images for pushes to main/develop
3. **Deploy** - Placeholder for production deployment

### Container Registry

Images are automatically published to GitHub Container Registry:

```bash
# Pull latest image
docker pull ghcr.io/username/rustdrop:latest

# Pull specific version
docker pull ghcr.io/username/rustdrop:v1.0.0
```

### Automatic Deployment

To enable automatic deployment:

1. Update the `deploy` job in `.github/workflows/ci-cd.yml`
2. Add your deployment logic (SSH, Kubernetes, etc.)
3. Configure necessary secrets in GitHub repository settings

Example deployment steps:

```yaml
- name: Deploy to production
  run: |
    ssh user@server 'docker pull ghcr.io/username/rustdrop:latest'
    ssh user@server 'docker-compose up -d'
```

## 🏥 Health Monitoring

### Health Check Endpoint

```bash
curl http://localhost:8080/api/health
```

Response:

```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00.000Z",
  "version": "0.1.0",
  "service": "rustdrop"
}
```

### Docker Health Check

Docker automatically monitors container health:

```bash
# Check container health
docker ps

# View health check logs
docker inspect --format='{{json .State.Health}}' <container_id>
```

## 📊 Production Considerations

### Security

- Run as non-root user (implemented in Dockerfile)
- Use HTTPS in production (reverse proxy recommended)
- Configure firewall rules
- Set appropriate file upload limits

### Performance

- Configure appropriate resource limits
- Use persistent volumes for file storage
- Monitor disk space for uploads
- Consider cleanup policies for old files

### Monitoring

- Health check endpoint for load balancers
- Container logs for debugging
- File system monitoring for storage

### Scaling

- Stateless design allows horizontal scaling
- Use shared storage for multi-instance deployments
- Load balancer with health checks
- Consider session affinity for WebUI

## 🔧 Troubleshooting

### Common Issues

1. **Port already in use**

   - Change RUSTDROP_SERVER\_\_PORT environment variable
   - Application auto-discovers available ports when running locally

2. **Permission denied for uploads**

   - Ensure /app/files directory is writable
   - Check Docker volume permissions

3. **Health check fails**

   - Verify application is running on correct port
   - Check container logs: `docker logs <container_id>`

4. **mDNS not working**
   - Disable with RUSTDROP_DISCOVERY\_\_ENABLED=false
   - mDNS requires network privileges in some environments
