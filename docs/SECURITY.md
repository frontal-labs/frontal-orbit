# Security Guide

This guide covers security aspects of the Orbit CLI, including permissions, sandboxing, data protection, and best practices.

## Security Overview

Orbit is designed with security as a primary concern, implementing multiple layers of protection:

- **Permission System** - Granular control over tool access
- **Sandboxing** - Isolated execution environments
- **Authentication** - Secure API key management
- **Data Protection** - Encryption and secure storage
- **Audit Logging** - Comprehensive activity tracking

## Permission System

### Permission Modes

#### danger-full-access
- **Description**: All tools are allowed without confirmation
- **Use Case**: Trusted environments, automation scripts
- **Risk**: High - full system access
- **Configuration**: `--permission-mode danger-full-access`

```bash
# Use with caution in trusted environments
orbit --permission-mode danger-full-access prompt "deploy to production"
```

#### safe-mode
- **Description**: Only safe tools allowed, destructive tools require approval
- **Use Case**: Untrusted codebases, learning environments
- **Risk**: Medium - limited destructive capability
- **Configuration**: `--permission-mode safe-mode`

```bash
# Safe mode for untrusted projects
orbit --permission-mode safe-mode prompt "analyze this codebase"
```

#### ask-permissions
- **Description**: Prompt for approval on every tool use
- **Use Case**: Maximum security, learning, debugging
- **Risk**: Low - explicit approval required
- **Configuration**: `--permission-mode ask-permissions`

```bash
# Maximum security
orbit --permission-mode ask-permissions prompt "list files in /tmp"
```

### Tool Permissions

#### Safe Tools (always allowed)
- `read` - Read file contents
- `grep` - Search file contents
- `glob` - Search file patterns
- `web_search` - Search the web
- `web_fetch` - Fetch web content

#### Restricted Tools (require approval)
- `write` - Write/create files
- `edit` - Edit existing files
- `bash` - Execute shell commands
- `agent` - Launch sub-agents

#### Dangerous Tools (high risk)
- `bash` with system commands
- `write` to system directories
- `edit` configuration files
- `agent` with full access

### Permission Configuration

```json
{
  "permissions": {
    "mode": "safe-mode",
    "allowed_tools": ["read", "grep", "web_search"],
    "restricted_tools": ["write", "edit"],
    "blocked_tools": ["bash"],
    "tool_restrictions": {
      "bash": {
        "allowed_commands": ["ls", "cat", "grep"],
        "blocked_commands": ["rm", "sudo", "chmod", "chown"],
        "allowed_paths": ["/tmp", "/home/user/projects"],
        "blocked_paths": ["/etc", "/usr/bin", "/bin"]
      },
      "write": {
        "allowed_paths": ["/tmp", "./", "/home/user/projects"],
        "blocked_paths": ["/etc", "/usr", "/bin"],
        "max_file_size": "10MB"
      },
      "edit": {
        "allowed_extensions": [".txt", ".md", ".js", ".py"],
        "blocked_extensions": [".sh", ".conf", ".key"],
        "backup_enabled": true
      }
    },
    "time_restrictions": {
      "allowed_hours": "9-17",
      "allowed_days": "mon-fri",
      "timezone": "UTC"
    }
  }
}
```

## Sandboxing

### Process Sandboxing

Orbit isolates tool execution in separate processes:

```bash
# Enable sandbox mode
orbit --sandbox enable

# Configure sandbox limits
orbit --sandbox --cpu-limit 50% --memory-limit 1GB
```

### Sandbox Configuration

```json
{
  "sandbox": {
    "enabled": true,
    "limits": {
      "cpu": "50%",
      "memory": "1GB",
      "disk": "100MB",
      "network": "restricted",
      "processes": 10
    },
    "filesystem": {
      "read_only": ["/usr", "/lib", "/etc"],
      "read_write": ["/tmp", "./"],
      "hidden": ["/home/user/.ssh", "/etc/ssl"]
    },
    "network": {
      "allowed_hosts": ["api.anthropic.com", "github.com"],
      "blocked_hosts": ["malicious.example.com"],
      "allowed_ports": [443, 80],
      "blocked_ports": [22, 23, 3389]
    }
  }
}
```

### Container Sandboxing

For maximum isolation, use container-based sandboxing:

```bash
# Enable container sandbox
orbit --sandbox container

# Use a pinned container image
orbit --sandbox container --image orbit-sandbox:v0.1.0
```

## Authentication and API Keys

### API Key Management

#### Environment Variables (Recommended)

```bash
# Set API keys in environment
export ORBIT_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
export XAI_API_KEY="xai-..."

# Use with Orbit
orbit prompt "analyze this code"
```

#### Config File Storage

```json
{
  "providers": {
    "anthropic": {
      "api_key": "${ORBIT_API_KEY}",
      "base_url": "https://api.anthropic.com"
    }
  }
}
```

#### Key Rotation

```bash
# Rotate API keys
orbit config rotate-api-keys anthropic

# Check key expiration
orbit config check-api-keys

# Set key expiration reminder
orbit config set key-expiry-reminder 7d
```

### API Security

#### Key Validation

```bash
# Validate API key
orbit auth validate anthropic

# Test API connectivity
orbit auth test anthropic

# Show key info (without exposing key)
orbit auth info anthropic
```

#### Rate Limiting

```json
{
  "rate_limiting": {
    "enabled": true,
    "requests_per_minute": 60,
    "tokens_per_minute": 100000,
    "burst_limit": 10,
    "backoff_strategy": "exponential"
  }
}
```

## Data Protection

### Encryption

#### Data at Rest

```json
{
  "encryption": {
    "enabled": true,
    "algorithm": "AES-256-GCM",
    "key_derivation": "PBKDF2",
    "iterations": 100000,
    "encrypt_sessions": true,
    "encrypt_config": false,
    "encrypt_cache": true
  }
}
```

#### Data in Transit

```json
{
  "tls": {
    "enabled": true,
    "version": "1.3",
    "cipher_suites": ["TLS_AES_256_GCM_SHA384"],
    "certificate_verification": true,
    "hsts": true
  }
}
```

### Sensitive Data Handling

#### Data Sanitization

```bash
# Enable data sanitization
orbit --sanitize-data prompt "process this file"

# Configure sanitization rules
orbit config set sanitize-patterns "password,token,key,secret"
```

#### Data Retention

```json
{
  "data_retention": {
    "sessions": "30d",
    "cache": "7d",
    "logs": "90d",
    "telemetry": "30d",
    "auto_cleanup": true
  }
}
```

### Privacy Settings

```json
{
  "privacy": {
    "disable_telemetry": true,
    "disable_usage_stats": true,
    "disable_error_reporting": false,
    "anonymize_data": true,
    "data_minimization": true
  }
}
```

## Audit and Logging

### Activity Logging

```json
{
  "logging": {
    "level": "info",
    "audit_log": {
      "enabled": true,
      "file": "~/.orbit/logs/audit.log",
      "format": "json",
      "rotation": "daily",
      "retention": "90d"
    },
    "events": [
      "tool_execution",
      "file_access",
      "network_request",
      "authentication",
      "permission_change",
      "config_change"
    ]
  }
}
```

### Security Events

```bash
# View security events
orbit audit security

# Show recent activity
orbit audit recent --hours 24

# Filter by event type
orbit audit filter --event tool_execution

# Export audit log
orbit audit export --format csv --output audit.csv
```

### Incident Response

```bash
# Lock down system on security event
orbit security lock

# Revoke all sessions
orbit security revoke-sessions

# Reset permissions to safe mode
orbit security reset-permissions

# Generate security report
orbit security report
```

## Network Security

### Network Restrictions

```json
{
  "network": {
    "allowed_hosts": [
      "api.anthropic.com",
      "api.openai.com",
      "api.x.ai",
      "github.com"
    ],
    "blocked_hosts": [
      "*.malicious.com",
      "phishing.example.com"
    ],
    "allowed_ports": [443, 80],
    "blocked_ports": [22, 23, 3389, 5432],
    "dns_servers": ["8.8.8.8", "1.1.1.1"],
    "proxy": {
      "enabled": false,
      "host": "",
      "port": 0,
      "auth": {
        "username": "",
        "password": ""
      }
    }
  }
}
```

### Certificate Validation

```json
{
  "certificates": {
    "validation": true,
    "custom_ca": [],
    "client_cert": {
      "enabled": false,
      "path": "",
      "key_path": ""
    },
    "ocsp_stapling": true,
    "certificate_pinning": false
  }
}
```

## Plugin Security

### Plugin Permissions

```json
{
  "plugins": {
    "permissions": {
      "default": "restricted",
      "require_explicit_approval": true,
      "sandbox_plugins": true,
      "signature_verification": true
    },
    "allowed_sources": [
      "https://github.com",
      "https://github.com/frontal-labs/frontal-orbit"
    ],
    "blocked_sources": [
      "*.malicious.com"
    ]
  }
}
```

### Plugin Sandboxing

```bash
# Enable plugin sandboxing
orbit config set plugin-sandbox true

# Configure plugin limits
orbit config set plugin-cpu-limit 25%
orbit config set plugin-memory-limit 512MB
```

### Plugin Verification

```bash
# Verify plugin signature
orbit plugin verify my-plugin

# Check plugin security
orbit plugin security-check my-plugin

# List trusted plugins
orbit plugin trusted
```

## Security Best Practices

### General Guidelines

1. **Use least privilege** - Grant minimum necessary permissions
2. **Regular updates** - Keep Orbit and dependencies updated
3. **Monitor activity** - Review audit logs regularly
4. **Secure storage** - Protect API keys and sensitive data
5. **Network security** - Restrict network access when possible

### Environment Security

```bash
# Use dedicated user account
useradd -m orbit
su - orbit

# Set restrictive file permissions
chmod 700 ~/.orbit
chmod 600 ~/.orbit/config.json

# Use secure shell
ssh -i ~/.ssh/orbit_key user@server
```

### Development Security

```bash
# Use safe mode for development
orbit --permission-mode safe-mode

# Enable audit logging
orbit --audit-log enable

# Use containerized development
docker run -it --rm orbit/cli:v0.1.0
```

### Production Security

```bash
# Use container sandbox
orbit --sandbox container

# Enable all security features
orbit --permission-mode ask-permissions --audit-log enable

# Monitor security events
orbit security monitor
```

## Security Configuration

### Security Hardening

```json
{
  "security": {
    "hardening": {
      "disable_debug_features": true,
      "disable_dev_tools": true,
      "enable_aslr": true,
      "enable_stack_protection": true,
      "disable_core_dumps": true
    },
    "intrusion_detection": {
      "enabled": true,
      "alert_threshold": 5,
      "block_threshold": 10,
      "alert_methods": ["email", "slack"]
    }
  }
}
```

### Compliance Settings

```json
{
  "compliance": {
    "standards": ["SOC2", "ISO27001", "GDPR"],
    "data_classification": "confidential",
    "audit_frequency": "daily",
    "retention_policy": "7y",
    "encryption_required": true
  }
}
```

## Threat Modeling

### Common Threats

1. **API Key Exposure** - Compromised authentication credentials
2. **Code Injection** - Malicious code execution through tools
3. **Data Exfiltration** - Unauthorized data access
4. **Privilege Escalation** - Gaining elevated system access
5. **Denial of Service** - Resource exhaustion attacks

### Mitigation Strategies

```json
{
  "threat_mitigation": {
    "api_key_exposure": {
      "rotation": "weekly",
      "encryption": true,
      "access_logging": true
    },
    "code_injection": {
      "input_validation": true,
      "sandboxing": true,
      "code_scanning": true
    },
    "data_exfiltration": {
      "egress_filtering": true,
      "data_loss_prevention": true,
      "access_controls": true
    }
  }
}
```

## Security Tools

### Built-in Security Tools

```bash
# Security scan
orbit security scan

# Vulnerability check
orbit security check-vulnerabilities

# Permission audit
orbit security audit-permissions

# Configuration security
orbit security check-config
```

### External Security Tools

```bash
# Integrate with security scanners
orbit security integrate --tool semgrep
orbit security integrate --tool trivy
orbit security integrate --tool bandit
```

## Incident Response

### Security Incident Types

1. **Unauthorized Access** - Suspicious login attempts
2. **Data Breach** - Unauthorized data access
3. **Malware Detection** - Suspicious code execution
4. **System Compromise** - System integrity issues

### Response Procedures

```bash
# Immediate response
orbit security incident --type unauthorized_access --action lock

# Investigation
orbit security investigate --incident-id 12345

# Recovery
orbit security recover --backup-id latest

# Post-incident review
orbit security review --incident-id 12345
```

## Security Updates

### Update Management

```bash
# Check for security updates
orbit security check-updates

# Apply security patches
orbit security update

# Verify update integrity
orbit security verify-update
```

### Security Advisories

```bash
# List security advisories
orbit security advisories

# Check specific vulnerability
orbit security advisory CVE-2024-12345

# Subscribe to security alerts
orbit security subscribe --email security@example.com
```

## Compliance and Auditing

### Compliance Reports

```bash
# Generate compliance report
orbit compliance report --standard SOC2

# Audit trail
orbit compliance audit-trail --start-date 2024-01-01

# Evidence collection
orbit compliance evidence --framework ISO27001
```

### Regulatory Compliance

```json
{
  "compliance": {
    "gdpr": {
      "data_processing": true,
      "consent_management": true,
      "data_subject_rights": true,
      "breach_notification": true
    },
    "soc2": {
      "security": true,
      "availability": true,
      "processing_integrity": true,
      "confidentiality": true,
      "privacy": true
    }
  }
}
```

This security guide provides comprehensive coverage of security features and best practices for using Orbit CLI safely in various environments.
