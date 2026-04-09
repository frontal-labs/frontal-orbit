# Troubleshooting Guide

This guide covers common issues, debugging techniques, and solutions for problems you might encounter with the Orbit CLI.

## Getting Help

### Built-in Help

```bash
# General help
orbit --help

# Command-specific help
orbit help prompt
orbit help repl
orbit help status

# Slash command help
/help
/status
/doctor
```

### Diagnostic Tools

```bash
# System diagnostics
orbit doctor

# Health check
orbit health check

# Configuration validation
orbit config validate

# Performance diagnostics
orbit diagnose performance
```

## Common Issues

### Installation and Setup

#### Problem: Cargo build fails

**Symptoms:**
```
error: failed to compile `orbit-cli v0.1.0`
error: could not compile `orbit-cli`
```

**Solutions:**
```bash
# Update Rust toolchain
rustup update stable

# Clear cargo cache
cargo clean

# Rebuild with verbose output
cargo build --workspace --verbose

# Check for missing dependencies
cargo check --workspace
```

#### Problem: Command not found

**Symptoms:**
```
zsh: command not found: orbit
```

**Solutions:**
```bash
# Install with Homebrew
brew install --HEAD ./homebrew/orbit.rb

# Verify Homebrew's bin directory is on PATH
eval "$(brew shellenv)"

# Use cargo run directly
cargo run -p orbit-cli -- --help
```

#### Problem: Permission denied

**Symptoms:**
```
Permission denied: ~/.orbit/config.json
```

**Solutions:**
```bash
# Create orbit directory with proper permissions
mkdir -p ~/.orbit
chmod 700 ~/.orbit

# Fix file permissions
chmod 600 ~/.orbit/config.json
chmod 700 ~/.orbit/sessions

# Check ownership
ls -la ~/.orbit
```

### Authentication Issues

#### Problem: API key not found

**Symptoms:**
```
Error: ANTHROPIC_API_KEY not found
```

**Solutions:**
```bash
# Set environment variable
export ANTHROPIC_API_KEY="sk-ant-..."

# Add to shell profile
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.zshrc

# Use config file
orbit config set providers.anthropic.api_key "sk-ant-..."

# Verify key is set
orbit auth validate anthropic
```

#### Problem: Invalid API key

**Symptoms:**
```
Error: Invalid API key
```

**Solutions:**
```bash
# Verify API key format
echo $ANTHROPIC_API_KEY | grep -E "^sk-ant-"

# Test API connectivity
orbit auth test anthropic

# Regenerate API key
# Visit https://console.anthropic.com/

# Check for typos
orbit config show providers.anthropic
```

#### Problem: Rate limited

**Symptoms:**
```
Error: Rate limit exceeded
```

**Solutions:**
```bash
# Check rate limits
orbit auth limits anthropic

# Wait and retry
sleep 60
orbit prompt "test message"

# Use different model
orbit --model claude-haiku-4-5-20251213 prompt "test"

# Configure rate limiting
orbit config set rate_limiting.requests_per_minute 30
```

### Network Issues

#### Problem: Connection timeout

**Symptoms:**
```
Error: Connection timeout
```

**Solutions:**
```bash
# Check network connectivity
ping api.anthropic.com

# Test with curl
curl -I https://api.anthropic.com

# Configure proxy
export HTTPS_PROXY=http://proxy.example.com:8080
export HTTP_PROXY=http://proxy.example.com:8080

# Increase timeout
orbit config set api.timeout 600

# Use different endpoint
orbit config set providers.anthropic.base_url "https://api.anthropic.com"
```

#### Problem: DNS resolution failed

**Symptoms:**
```
Error: DNS resolution failed
```

**Solutions:**
```bash
# Check DNS resolution
nslookup api.anthropic.com
dig api.anthropic.com

# Use different DNS server
export DNS_SERVERS="8.8.8.8,1.1.1.1"

# Flush DNS cache
sudo dscacheutil -flushcache

# Configure DNS in Orbit
orbit config set network.dns_servers "8.8.8.8,1.1.1.1"
```

### Performance Issues

#### Problem: Slow response times

**Symptoms:**
- Commands take >30 seconds to respond
- High CPU usage
- Memory consumption grows

**Solutions:**
```bash
# Check system resources
orbit resources monitor

# Optimize configuration
orbit config set runtime.cache_size "200MB"
orbit config set api.connection_pool.max_connections 5

# Use faster model
orbit --model haiku prompt "quick test"

# Enable caching
orbit config set caching.memory.enabled true

# Profile performance
orbit profile cpu --duration 30s
```

#### Problem: Memory leaks

**Symptoms:**
- Memory usage increases over time
- System becomes unresponsive
- Out of memory errors

**Solutions:**
```bash
# Monitor memory usage
orbit memory monitor

# Reduce cache sizes
orbit config set caching.memory.max_size "50MB"
orbit config set runtime.memory_limit "1GB"

# Enable garbage collection
orbit config set runtime.gc_interval "30s"

# Restart Orbit
pkill orbit-cli
orbit prompt "test"

# Memory profile
orbit profile memory --duration 60s
```

### Tool Issues

#### Problem: Tool execution failed

**Symptoms:**
```
Error: Tool 'bash' execution failed
```

**Solutions:**
```bash
# Check tool permissions
orbit config show permissions

# Test tool manually
orbit tool test bash --command "echo test"

# Check tool availability
orbit tools list

# Enable tool
orbit config set permissions.allowed_tools "bash,read,write"

# Debug tool execution
orbit debug tool bash --command "ls -la"
```

#### Problem: File access denied

**Symptoms:**
```
Error: Permission denied: /etc/hosts
```

**Solutions:**
```bash
# Check file permissions
ls -la /etc/hosts

# Use safe mode
orbit --permission-mode safe-mode prompt "read /etc/hosts"

# Configure allowed paths
orbit config set permissions.tool_restrictions.bash.allowed_paths "/tmp,./"

# Run with elevated privileges (caution)
sudo orbit prompt "read /etc/hosts"
```

### Session Issues

#### Problem: Session not found

**Symptoms:**
```
Error: Session 'session-123' not found
```

**Solutions:**
```bash
# List available sessions
orbit session list

# Resume latest session
orbit --resume latest

# Check session directory
ls -la ~/.orbit/sessions

# Create new session
orbit prompt "start new session"

# Export session
orbit session export --session session-123 --output session.json
```

#### Problem: Session corruption

**Symptoms:**
```
Error: Session file corrupted
```

**Solutions:**
```bash
# Validate session
orbit session validate --session session-123

# Repair session
orbit session repair --session session-123

# Clear corrupted sessions
orbit session clean --corrupted

# Start fresh session
orbit prompt "new session after corruption"
```

### Plugin Issues

#### Problem: Plugin fails to load

**Symptoms:**
```
Error: Plugin 'my-plugin' failed to load
```

**Solutions:**
```bash
# Check plugin status
orbit plugin list

# Validate plugin
orbit plugin validate my-plugin

# Check dependencies
orbit plugin dependencies my-plugin

# Reinstall plugin
orbit plugin uninstall my-plugin
orbit plugin install my-plugin

# Debug plugin loading
orbit debug plugin my-plugin
```

#### Problem: Plugin permission denied

**Symptoms:**
```
Error: Plugin permission denied
```

**Solutions:**
```bash
# Check plugin permissions
orbit plugin permissions my-plugin

# Grant required permissions
orbit plugin grant my-plugin network

# Configure plugin sandbox
orbit config set plugins.sandbox false

# Review plugin manifest
cat ~/.orbit/plugins/my-plugin/plugin.json
```

### MCP Issues

#### Problem: MCP server not running

**Symptoms:**
```
Error: MCP server 'filesystem' not running
```

**Solutions:**
```bash
# Check MCP server status
orbit mcp status filesystem

# Start MCP server
orbit mcp start filesystem

# Check server configuration
orbit mcp config show filesystem

# Debug server startup
orbit debug mcp filesystem

# Restart server
orbit mcp restart filesystem
```

#### Problem: MCP tools not available

**Symptoms:**
```
Error: Tool 'filesystem/read' not found
```

**Solutions:**
```bash
# List available MCP tools
orbit mcp tools

# Check server tools
orbit mcp tools filesystem

# Test server connection
orbit mcp test filesystem

# Reload server tools
orbit mcp reload filesystem

# Check server logs
orbit mcp logs filesystem
```

## Debugging Techniques

### Enable Debug Logging

```bash
# Set debug log level
export RUST_LOG=debug

# Enable specific module debugging
export RUST_LOG=orbit::cli=debug,orbit::runtime=info

# Run with debug output
RUST_LOG=debug orbit prompt "test message"

# Save debug logs to file
RUST_LOG=debug orbit prompt "test" 2>&1 | tee debug.log
```

### Verbose Mode

```bash
# Run with verbose output
orbit --verbose prompt "test"

# Extra verbose mode
orbit --verbose --verbose prompt "test"

# Show configuration
orbit config show --verbose
```

### Dry Run Mode

```bash
# Test command without execution
orbit --dry-run prompt "delete all files"

# Validate configuration
orbit config validate --dry-run

# Test plugin installation
orbit plugin install --dry-run my-plugin
```

### Step-by-Step Debugging

```bash
# Enable step-by-step mode
orbit --step-by-step prompt "complex task"

# Interactive debugging
orbit debug interactive

# Break on errors
orbit debug --break-on-error prompt "risky operation"
```

## Error Codes

### Common Error Codes

| Code | Description | Solution |
|------|-------------|----------|
| 1 | General error | Check logs for details |
| 2 | Configuration error | Validate config file |
| 3 | Authentication error | Check API keys |
| 4 | Network error | Check network connectivity |
| 5 | Permission error | Check file permissions |
| 6 | Tool execution error | Validate tool configuration |
| 7 | Session error | Check session files |
| 8 | Plugin error | Validate plugin installation |
| 9 | MCP error | Check MCP server status |
| 10 | Resource error | Check system resources |

### Error Details

```bash
# Show error details
orbit error show 12345

# Error lookup
orbit error lookup "permission denied"

# Error troubleshooting
orbit troubleshoot --error-code 4
```

## System Diagnostics

### Health Check

```bash
# Comprehensive health check
orbit health check --comprehensive

# Quick health check
orbit health check --quick

# Specific component check
orbit health check --component api
orbit health check --component tools
orbit health check --component mcp
```

### System Information

```bash
# Show system info
orbit system info

# Show configuration
orbit config show

# Show environment
orbit env show

# Show version info
orbit version --verbose
```

### Performance Diagnostics

```bash
# Performance check
orbit performance check

# Resource usage
orbit resources usage

# Bottleneck analysis
orbit analyze bottlenecks

# Optimization suggestions
orbit optimize suggest
```

## Getting Support

### Community Support

```bash
# Generate support bundle
orbit support bundle --output support-bundle.tar.gz

# Check for known issues
orbit issues search "connection timeout"

# Report issue
orbit issue report --type bug --description "Detailed description"
```

### Contact Support

```bash
# Generate diagnostic report
orbit diagnostics report --output diagnostics.json

# Export configuration
orbit config export --output config.json

# Export logs
orbit logs export --days 7 --output logs.tar.gz
```

## Recovery Procedures

### Configuration Recovery

```bash
# Reset configuration
orbit config reset

# Restore from backup
orbit config restore --backup config-backup.json

# Initialize default configuration
orbit config init --defaults

# Validate configuration
orbit config validate
```

### Session Recovery

```bash
# List corrupted sessions
orbit session list --corrupted

# Repair sessions
orbit session repair --all

# Export sessions
orbit session export --all

# Clear sessions
orbit session clear --all
```

### Plugin Recovery

```bash
# List broken plugins
orbit plugin list --broken

# Reinstall all plugins
orbit plugin reinstall --all

# Reset plugin registry
orbit plugin registry reset

# Validate plugins
orbit plugin validate --all
```

## Prevention Tips

### Regular Maintenance

```bash
# Clean up old sessions
orbit session cleanup --older-than 30d

# Clear cache
orbit cache clear --all

# Update plugins
orbit plugin update --all

# Check system health
orbit health check
```

### Monitoring

```bash
# Enable monitoring
orbit monitoring enable

# Set up alerts
orbit alerts enable --type error

# Performance monitoring
orbit performance monitor

# Resource monitoring
orbit resources monitor
```

### Backup Strategies

```bash
# Backup configuration
orbit config backup --output config-backup.json

# Backup sessions
orbit session backup --output sessions-backup.tar.gz

# Backup plugins
orbit plugin backup --output plugins-backup.tar.gz

# Automated backup
orbit backup schedule --daily --retain 7
```

This troubleshooting guide provides comprehensive coverage of common issues and solutions for using Orbit CLI effectively.
