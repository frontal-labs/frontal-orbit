# Orbit Slack Extension Troubleshooting Guide

## Overview

This guide provides comprehensive troubleshooting steps for common issues with the Orbit Slack extension.

## Quick Diagnosis

### Health Check

First, verify the extension is running:

```bash
curl http://localhost:3000/health
```

Expected response:
```json
{
  "status": "ok",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0"
}
```

### Log Levels

Adjust log level for debugging:

```bash
# Development
LOG_LEVEL=debug bun run dev

# Production
LOG_LEVEL=debug bun start
```

## Common Issues

### 1. Bot Not Responding

#### Symptoms
- Bot doesn't respond to commands
- No messages posted in channels
- Slash commands return "Sorry, something went wrong"

#### Diagnosis Steps

1. **Check Bot Token**
   ```bash
   # Verify token format
   echo $SLACK_BOT_TOKEN | grep -E "^xoxb-[0-9]+-[0-9]+"
   ```

2. **Check Bot Permissions**
   - Verify bot is invited to the channel
   - Check required scopes in Slack app settings
   - Ensure bot has posting permissions

3. **Check Server Logs**
   ```bash
   # Look for authentication errors
   grep "authentication" logs/app.log | tail -10
   
   # Check for Slack API errors
   grep "slack.*error" logs/app.log | tail -10
   ```

4. **Test Slack Connection**
   ```bash
   # Test API connectivity
   curl -H "Authorization: Bearer $SLACK_BOT_TOKEN" \
        https://slack.com/api/auth.test
   ```

#### Solutions

**Invalid Bot Token:**
```bash
# Generate new bot token in Slack app settings
# Update .env file
SLACK_BOT_TOKEN=xoxb-new-token-here
# Restart service
docker restart orbit-slack
```

**Missing Permissions:**
- Add required scopes: `commands`, `chat:write`, `chat:write.public`, `users:read`, `channels:read`
- Invite bot to channel: `/invite @orbit-bot`
- Reinstall app if needed

**Network Issues:**
```bash
# Check firewall
sudo ufw status | grep 3000

# Test Slack API connectivity
ping api.slack.com
telnet api.slack.com 443
```

### 2. Tasks Not Creating

#### Symptoms
- `/ai` commands don't create tasks
- No response from task creation
- Errors in task creation flow

#### Diagnosis Steps

1. **Check Orbit API Connection**
   ```bash
   # Test API connectivity
   curl -f $ORBIT_API_URL/health
   
   # Check API timeout
   curl -w "@curl-format.txt" -o /dev/null -s "$ORBIT_API_URL/health"
   ```

2. **Verify API Configuration**
   ```bash
   # Check API URL format
   echo $ORBIT_API_URL | grep -E "^https?://"
   
   # Test with curl
   curl -H "Content-Type: application/json" \
        -d '{"prompt":"test","source":"troubleshoot"}' \
        "$ORBIT_API_URL/tasks"
   ```

3. **Check Request Logs**
   ```bash
   # Look for API request logs
   grep "POST.*tasks" logs/app.log | tail -5
   
   # Check for timeout errors
   grep "timeout" logs/app.log | tail -5
   ```

4. **Verify Request Format**
   ```bash
   # Check Slack command parsing
   grep "slash.*command" logs/app.log | tail -5
   ```

#### Solutions

**API Connection Issues:**
```bash
# Update API URL in .env
ORBIT_API_URL=https://orbit-api.your-domain.com

# Increase timeout
ORBIT_API_TIMEOUT=60000

# Restart service
systemctl restart orbit-slack
```

**Invalid Request Format:**
```typescript
// Verify request structure
const request = {
  prompt: "Fix the login bug",
  source: "slack",
  priority: "medium"
};
```

**Authentication Issues:**
```bash
# Add API token if required
ORBIT_API_TOKEN=your-api-token-here
```

### 3. WebSocket Connection Issues

#### Symptoms
- Task updates not appearing in Slack
- Delayed or missing notifications
- WebSocket connection errors

#### Diagnosis Steps

1. **Check WebSocket Logs**
   ```bash
   # Look for WebSocket connection logs
   grep "websocket" logs/app.log | tail -10
   
   # Check for disconnection events
   grep "disconnect" logs/app.log | tail -10
   ```

2. **Verify Socket Mode Configuration**
   ```bash
   # Check app token format
   echo $SLACK_APP_TOKEN | grep -E "^xapp-[0-9]+-[0-9]+"
   
   # Test Socket Mode connection
   curl -H "Authorization: Bearer $SLACK_APP_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"debug":true}' \
        https://slack.com/api/apps.connections.open
   ```

3. **Check Network Connectivity**
   ```bash
   # Test WebSocket endpoint
   telnet wss-primary.slack.com 443
   
   # Check firewall rules
   sudo iptables -L | grep 443
   ```

4. **Monitor Connection State**
   ```bash
   # Watch connection logs in real-time
   tail -f logs/app.log | grep -E "(websocket|connection)"
   ```

#### Solutions

**Invalid App Token:**
```bash
# Generate new app token with Socket Mode
# Update .env file
SLACK_APP_TOKEN=xapp-new-token-here
# Restart service
docker restart orbit-slack
```

**Network Restrictions:**
```bash
# Allow WebSocket traffic
sudo ufw allow out 443/tcp
sudo ufw allow out wss-primary.slack.com

# Add WebSocket proxy if behind corporate firewall
```

**Connection Limits:**
```typescript
// Implement connection retry logic
const maxRetries = 5;
const retryDelay = 5000;

async function connectWithRetry() {
  for (let i = 0; i < maxRetries; i++) {
    try {
      await client.connect();
      return;
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await new Promise(resolve => setTimeout(resolve, retryDelay));
    }
  }
}
```

### 4. Approval Workflows Not Working

#### Symptoms
- Approval buttons not appearing
- Button clicks not working
- Orphan approvals not resolving

#### Diagnosis Steps

1. **Check Interactive Component Logs**
   ```bash
   # Look for interactive component logs
   grep "interactive" logs/app.log | tail -10
   
   # Check for action handling logs
   grep "action.*handler" logs/app.log | tail -10
   ```

2. **Verify Response URL**
   ```bash
   # Check response URL in logs
   grep "response_url" logs/app.log | tail -5
   ```

3. **Test Button Interaction**
   ```bash
   # Look for block action logs
   grep "block_actions" logs/app.log | tail -5
   ```

4. **Check Approval API Calls**
   ```bash
   # Look for approval resolution logs
   grep "approval.*resolve" logs/app.log | tail -5
   ```

#### Solutions

**Missing Interactive Components:**
```bash
# Enable interactive components in Slack app
# Add "message" and "block_actions" to Interactive Components
# Reinstall app if needed
```

**Response URL Timeout:**
```typescript
// Handle response URL expiration
async function sendResponse(responseUrl: string, message: any) {
  try {
    await axios.post(responseUrl, message);
  } catch (error) {
    if (error.response?.status === 404) {
      // Response URL expired, use chat.postMessage instead
      await slackClient.chat.postMessage({
        channel: originalChannel,
        text: "Action completed"
      });
    }
  }
}
```

**Permission Issues:**
- Verify bot has `chat:write` scope
- Check bot is in the channel
- Ensure interactive components are enabled

### 5. High Memory Usage

#### Symptoms
- Container OOM kills
- Slow performance
- Memory leaks

#### Diagnosis Steps

1. **Monitor Memory Usage**
   ```bash
   # Check current memory usage
   ps aux | grep orbit-slack
   
   # Monitor memory over time
   watch -n 5 'ps aux | grep orbit-slack'
   ```

2. **Check Memory Logs**
   ```bash
   # Look for memory warnings
   grep "memory" logs/app.log | tail -10
   
   # Check for garbage collection logs
   grep "gc" logs/app.log | tail -10
   ```

3. **Profile Memory**
   ```bash
   # Enable heap profiling
   NODE_OPTIONS="--inspect" bun run dev
   
   # Use Chrome DevTools for memory profiling
   chrome://inspect
   ```

4. **Check for Leaks**
   ```bash
   # Monitor memory growth
   while true; do
     echo "$(date): $(ps -o pid,vsz,rss,comm -p $(pgrep orbit-slack))"
     sleep 30
   done
   ```

#### Solutions

**Increase Memory Limits:**
```yaml
# Docker
resources:
  limits:
    memory: "1Gi"
  requests:
    memory: "512Mi"

# Kubernetes
resources:
  limits:
    memory: "1Gi"
  requests:
    memory: "512Mi"
```

**Optimize Garbage Collection:**
```bash
# Set GC options
NODE_OPTIONS="--max-old-space-size=512 --gc-interval=100"

# Enable GC logging
NODE_OPTIONS="--trace-gc"
```

**Fix Memory Leaks:**
```typescript
// Clean up event listeners
process.on('exit', () => {
  client.removeAllListeners();
});

// Use weak references for cached data
const WeakMap = require('weak-map');
const cache = new WeakMap();
```

### 6. Rate Limiting Issues

#### Symptoms
- Messages not posting
- API errors
- Throttling warnings

#### Diagnosis Steps

1. **Check Rate Limit Logs**
   ```bash
   # Look for rate limit errors
   grep "rate.*limit" logs/app.log | tail -10
   
   # Check for 429 responses
   grep "429" logs/app.log | tail -10
   ```

2. **Monitor API Usage**
   ```bash
   # Count API calls per minute
   grep "POST.*slack.com" logs/app.log | awk '{print $1}' | sort | uniq -c
   ```

3. **Check Slack Rate Limits**
   ```bash
   # Monitor response headers
   curl -I -H "Authorization: Bearer $SLACK_BOT_TOKEN" \
        https://slack.com/api/chat.postMessage
   ```

#### Solutions

**Implement Rate Limiting:**
```typescript
import rateLimit from 'express-rate-limit';

const limiter = rateLimit({
  windowMs: 60 * 1000, // 1 minute
  max: 100, // 100 requests per minute
  message: 'Too many requests'
});

app.use(limiter);
```

**Queue Messages:**
```typescript
class MessageQueue {
  private queue: Array<{message: any, resolve: Function}> = [];
  private processing = false;
  
  async enqueue(message: any): Promise<void> {
    return new Promise((resolve) => {
      this.queue.push({message, resolve});
      this.process();
    });
  }
  
  private async process() {
    if (this.processing) return;
    this.processing = true;
    
    while (this.queue.length > 0) {
      const {message, resolve} = this.queue.shift()!;
      try {
        await this.sendMessage(message);
        resolve();
      } catch (error) {
        // Retry logic
        this.queue.unshift({message, resolve});
        await new Promise(r => setTimeout(r, 1000));
      }
    }
    
    this.processing = false;
  }
}
```

### 7. Configuration Issues

#### Symptoms
- Environment variable errors
- Invalid configuration
- Service won't start

#### Diagnosis Steps

1. **Check Environment Variables**
   ```bash
   # Verify all required variables
   env | grep -E "^(SLACK|ORBIT|NODE_ENV)"
   
   # Check for missing variables
   comm -23 <(sort .env.example) <(sort .env)
   ```

2. **Validate Configuration**
   ```bash
   # Test configuration loading
   bun -e "console.log(require('./src/config'))"
   
   # Check for syntax errors
   bun -c src/config.ts
   ```

3. **Check File Permissions**
   ```bash
   # Verify .env file permissions
   ls -la .env
   
   # Check read permissions
   test -r .env && echo "Readable" || echo "Not readable"
   ```

#### Solutions

**Missing Environment Variables:**
```bash
# Copy example and fill in values
cp .env.example .env

# Edit with required values
nano .env

# Validate configuration
bun run config:validate
```

**Invalid Configuration:**
```typescript
// Add configuration validation
import Joi from 'joi';

const configSchema = Joi.object({
  SLACK_BOT_TOKEN: Joi.string().pattern(/^xoxb-/).required(),
  SLACK_APP_TOKEN: Joi.string().pattern(/^xapp-/).required(),
  ORBIT_API_URL: Joi.string().uri().required(),
  PORT: Joi.number().port().default(3000)
});

const { error } = configSchema.validate(process.env);
if (error) {
  throw new Error(`Configuration error: ${error.message}`);
}
```

## Advanced Troubleshooting

### Debug Mode

Enable comprehensive debugging:

```bash
# Enable all debug logs
LOG_LEVEL=debug NODE_OPTIONS="--trace-warnings" bun run dev

# Enable Node.js debugging
NODE_OPTIONS="--inspect-brk" bun run dev
```

### Network Debugging

Test network connectivity:

```bash
# Test DNS resolution
nslookup api.slack.com
nslookup orbit-api.your-domain.com

# Test TCP connectivity
telnet api.slack.com 443
telnet orbit-api.your-domain.com 8788

# Test HTTP connectivity
curl -v https://api.slack.com/api/auth.test
curl -v $ORBIT_API_URL/health
```

### Performance Profiling

Profile application performance:

```bash
# Enable CPU profiling
NODE_OPTIONS="--prof" bun run dev

# Analyze profile
node --prof-process isolate-*.log > performance.txt

# Generate flame graph
npm install -g 0x
0x --output-dir profiling/ bun start
```

### Memory Profiling

Profile memory usage:

```bash
# Enable heap profiling
NODE_OPTIONS="--inspect" bun run dev

# Generate heap snapshot
# Use Chrome DevTools: chrome://inspect

# Analyze heap dump
node --heap-prof start.js
```

## Getting Help

### Collect Debug Information

Create a debug bundle:

```bash
#!/bin/bash
# collect-debug-info.sh

DEBUG_DIR="debug-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$DEBUG_DIR"

# Collect logs
cp logs/app.log "$DEBUG_DIR/"
cp logs/error.log "$DEBUG_DIR/"

# Collect configuration
cp .env "$DEBUG_DIR/.env.redacted"  # Redact sensitive data
cp package.json "$DEBUG_DIR/"
cp bunfig.toml "$DEBUG_DIR/"

# Collect system info
uname -a > "$DEBUG_DIR/system-info"
docker version > "$DEBUG_DIR/docker-version"
node --version > "$DEBUG_DIR/node-version"
bun --version > "$DEBUG_DIR/bun-version"

# Collect metrics
curl http://localhost:3000/health > "$DEBUG_DIR/health-check"

# Create archive
tar -czf "$DEBUG_DIR.tar.gz" "$DEBUG_DIR"
echo "Debug info collected: $DEBUG_DIR.tar.gz"
```

### Log Analysis Scripts

Analyze common patterns:

```bash
#!/bin/bash
# analyze-logs.sh

LOG_FILE="logs/app.log"

# Error analysis
echo "=== ERROR ANALYSIS ==="
grep "ERROR" "$LOG_FILE" | tail -20

# Performance analysis
echo "=== PERFORMANCE ANALYSIS ==="
grep "duration" "$LOG_FILE" | tail -10

# API analysis
echo "=== API ANALYSIS ==="
grep "POST.*api" "$LOG_FILE" | tail -10

# WebSocket analysis
echo "=== WEBSOCKET ANALYSIS ==="
grep "websocket" "$LOG_FILE" | tail -10
```

### Contact Support

When requesting support, include:

1. **Version**: Extension version and commit hash
2. **Environment**: Development or production
3. **Logs**: Relevant log entries
4. **Configuration**: Redacted configuration
5. **Steps to Reproduce**: Detailed reproduction steps
6. **Expected vs Actual**: What you expected vs what happened

### Community Resources

- **GitHub Issues**: Report bugs and feature requests
- **Slack Community**: Get help from other users
- **Documentation**: Check latest documentation
- **Changelog**: Review recent changes for breaking updates

## Prevention

### Monitoring Setup

Set up proactive monitoring:

```yaml
# Prometheus alerts
groups:
- name: orbit-slack
  rules:
  - alert: HighErrorRate
    expr: rate(orbit_slack_errors_total[5m]) > 0.1
    for: 2m
    labels:
      severity: warning
    annotations:
      summary: "High error rate detected"
      
  - alert: HighMemoryUsage
    expr: orbit_slack_memory_usage_bytes > 500000000
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High memory usage detected"
```

### Health Checks

Implement comprehensive health checks:

```typescript
app.get('/health/detailed', async (req, res) => {
  const checks = {
    orbit_api: await checkOrbitAPI(),
    slack_connection: await checkSlackConnection(),
    websocket: checkWebSocketConnection(),
    memory: checkMemoryUsage(),
    disk: checkDiskUsage()
  };
  
  const healthy = Object.values(checks).every(check => check.status === 'ok');
  
  res.json({
    status: healthy ? 'ok' : 'error',
    timestamp: new Date().toISOString(),
    checks
  });
});
```

### Automated Testing

Set up automated testing:

```bash
#!/bin/bash
# health-check.sh

# Test basic connectivity
curl -f http://localhost:3000/health || exit 1

# Test Slack integration
curl -f http://localhost:3000/health/slack || exit 1

# Test Orbit API
curl -f http://localhost:3000/health/orbit || exit 1

echo "All health checks passed"
```

This comprehensive troubleshooting guide should help diagnose and resolve most common issues with the Orbit Slack extension.
