# Distributed Load Testing

**Status:** Draft / Not yet run against a live deployment

This directory contains **distributed load testing** infrastructure for the Contact Relationship Management service using [k6](https://k6.io/), an open-source load testing tool built for modern cloud-native applications.

## Quick Start

### Prerequisites

- [k6](https://k6.io/docs/get-started/installation/) (v0.50+ recommended)
- Running CRM service instance
- PostgreSQL 18 (for database tests)
- Redis 8 (for cache tests)

### Run a Basic Load Test

```bash
# From the monorepo root
cd tests/load

# Run the quick smoke test
k6 run scripts/smoke.test.js

# Run a basic load test
k6 run scripts/basic-load.test.js

# Run with cloud execution (distributed)
# Note: Requires k6 Cloud account
k6 cloud scripts/basic-load.test.js
```

### Run Distributed Tests

```bash
# Start a distributed test with multiple VUs across multiple instances
# Using Docker (recommended for isolation)
docker compose -f docker-compose.load-test.yaml up --scale k6=10

# Or manually with multiple k6 instances
k6 run --out influxdb=http://influxdb:8086/k6 scripts/distributed.test.js
```

## Test Scenarios

| Scenario | File | Purpose | Duration | VUs |
|----------|------|---------|----------|-----|
| Smoke | `scripts/smoke.test.js` | Verify basic functionality | 1-2 min | 1-5 |
| Basic Load | `scripts/basic-load.test.js` | Baseline performance | 5-10 min | 10-50 |
| Stress | `scripts/stress.test.js` | Find breaking points | 15-30 min | 100-500 |
| Soak | `scripts/soak.test.js` | Long-running stability | 1-24 hours | 20-100 |
| Spike | `scripts/spike.test.js` | Sudden traffic surge | 5-10 min | 0-1000 |
| API Contract | `scripts/api-contract.test.js` | Verify OpenAPI compliance | 5-10 min | 5-20 |
| Database | `scripts/database.test.js` | PostgreSQL performance | 10-30 min | 10-50 |
| Family Bridges | `scripts/bridges.test.js` | PPM/WPM integration | 10-15 min | 10-30 |

## Configuration

Environment-specific configurations are in the `config/` directory:

| File | Description |
|------|-------------|
| `config/base.js` | Base configuration (timeouts, thresholds) |
| `config/dev.js` | Development environment |
| `config/ci.js` | CI environment |
| `config/staging.js` | Staging environment |
| `config/production.js` | Production environment |

### Custom Configuration

Create a new configuration file or override via environment variables:

```javascript
// In your test script
import { base } from './config/base.js';

const config = {
  ...base,
  vus: 100,
  duration: '10m',
  baseUrl: process.env.CRM_BASE_URL || 'http://localhost:1241',
};
```

## Running Tests

### Local Development

```bash
# Start the service
podman compose up --build

# Run load tests in another terminal
cd tests/load
k6 run --env CRM_BASE_URL=http://localhost:1241 scripts/smoke.test.js
```

### CI/CD Integration

```bash
# Run in GitHub Actions (see .github/workflows/load-test.yml)
# Or run manually
scripts/ci-check.sh load-test
```

### Distributed Testing with Docker

```bash
# Start the service and dependencies
docker compose -f compose.yaml up -d

# Run distributed load test
docker compose -f docker-compose.load-test.yaml up --scale k6=5
```

## Metrics & Output

### Built-in Metrics

k6 automatically collects:

- HTTP requests (count, rate, duration)
- HTTP errors (4xx, 5xx)
- Data sent/received
- Checks passed/failed
- Custom metrics

### Output Options

```bash
# JSON output
k6 run --out json=results/result.json scripts/basic-load.test.js

# InfluxDB (for Grafana)
k6 run --out influxdb=http://localhost:8086/k6 scripts/basic-load.test.js

# Prometheus
k6 run --out experimental-prometheus-rw scripts/basic-load.test.js

# Cloud (k6 Cloud)
k6 cloud scripts/basic-load.test.js
```

### Custom Metrics

Define custom metrics in your test scripts:

```javascript
import { Trend, Rate, Counter } from 'k6/metrics';

const responseTimes = new Trend('response_times');
const errorRate = new Rate('error_rate');
const requestCount = new Counter('request_count');

// In your test
export default function () {
  const res = http.get('http://localhost:1241/api/contacts');
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  requestCount.add(1);
}
```

## Thresholds

Define pass/fail criteria for your tests:

```javascript
import { check } from 'k6';

export const options = {
  thresholds: {
    // HTTP errors should be < 1%
    http_req_failed: ['rate<0.01'],
    
    // 95% of requests should be below 500ms
    http_req_duration: ['p(95)<500'],
    
    // 99% of requests should be below 1000ms
    http_req_duration: ['p(99)<1000'],
    
    // Custom metric: response times should average < 300ms
    response_times: ['avg<300'],
  },
};
```

## Test Data

### Synthetic Data Generation

Use the `lib/` directory for reusable test data generators:

```javascript
import { generateContact, generateDeal, generateCampaign } from './lib/generators.js';

export default function () {
  const contact = generateContact();
  const deal = generateDeal(contact.pid);
  // ...
}
```

### Seed Data

For tests that require existing data, use the seed task:

```bash
# Seed the database with test data
cargo run -- task seed

# Or with specific parameters
cargo run -- task seed --count 1000 --users 50
```

## Best Practices

1. **Start Small**: Begin with smoke tests, then gradually increase load
2. **Monitor Resources**: Watch CPU, memory, and database connections
3. **Define Thresholds**: Always set pass/fail criteria
4. **Isolate Environments**: Don't test in production
5. **Test Gradually**: Ramp up load to find breaking points
6. **Monitor Dependencies**: Check PostgreSQL, Redis, and other services
7. **Document Results**: Save results for comparison over time

## Troubleshooting

### Common Issues

1. **Connection Refused**: Service not running or wrong URL
   - Check `CRM_BASE_URL` environment variable
   - Verify service is running: `curl http://localhost:1241/_health/live`

2. **Authentication Required**: Tests need auth token
   - Set `CRM_REQUIRE_AUTH=0` for testing
   - Or provide token: `export CRM_TOKEN=...`

3. **Database Connection Errors**: PostgreSQL not available
   - Start test database: `podman compose -f compose.test.yaml up -d`
   - Set `DB_URL=postgres://loco:loco@localhost:5432/contact_relationship_management_test`

4. **Rate Limiting**: Hitting rate limits
   - Increase rate limits in development
   - Or reduce VU count in tests

5. **Memory Issues**: k6 running out of memory
   - Reduce VU count
   - Run fewer concurrent tests

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| P95 Response Time | < 500ms | For simple requests |
| P99 Response Time | < 1000ms | For simple requests |
| Error Rate | < 0.1% | HTTP 4xx/5xx |
| Request Rate | 100+ req/s | Per instance |
| Concurrent Users | 1000+ | Supported |

## Results Analysis

After running tests, analyze results in:

- `results/` directory (JSON, CSV outputs)
- InfluxDB + Grafana (for time-series visualization)
- k6 Cloud (for distributed results)

### Key Metrics to Analyze

1. **Response Times**: Average, P95, P99
2. **Error Rates**: 4xx and 5xx errors
3. **Throughput**: Requests per second
4. **Resource Utilization**: CPU, memory
5. **Database Performance**: Query times, connection pool
6. **Cache Hit Rate**: Redis performance

## Integration with CI/CD

See `.github/workflows/load-test.yml` for automated load testing in CI.

### Running in CI

```yaml
# Example GitHub Actions workflow
- name: Load Test
  run: |
    cd tests/load
    k6 run --env CRM_BASE_URL=http://localhost:1241 scripts/smoke.test.js
```

## Distributed Testing

For large-scale distributed tests:

### Using k6 Cloud

```bash
# Install k6 Cloud CLI
k6 login cloud --token YOUR_TOKEN

# Run distributed test
k6 cloud --out cloud scripts/distributed.test.js
```

### Using Docker Swarm

```bash
# Create a Docker Swarm
docker swarm init

# Deploy k6 services
docker stack deploy -c docker-compose.load-test.yaml k6

# View results
docker service logs k6_k6
```

### Using Kubernetes

```yaml
# k6-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: k6
spec:
  replicas: 10
  template:
    spec:
      containers:
      - name: k6
        image: grafana/k6
        command: ["k6", "run", "--vus", "10", "--duration", "10m", "scripts/basic-load.test.js"]
```

## Related Documentation

- [Performance Testing Guide](docs/performance-testing.md) (roadmap)
- [Scaling Guide](docs/scaling.md) (roadmap)
- [CRM API Documentation](spec/) - OpenAPI specs
- [Architecture](spec/architecture.md) - System design

---

*This load testing infrastructure is a work in progress. Tests have not yet been run against a live deployment. See [spec/tasks.md](../spec/tasks.md) for the live delivery checklist.*
