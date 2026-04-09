# Examples Guide

This guide provides practical examples of using Orbit CLI for various tasks and workflows.

## Quick Start Examples

### Basic Usage

```bash
# Simple prompt
orbit prompt "What files are in the current directory?"

# Read and analyze a file
orbit prompt "Read the README.md file and summarize it"

# Code analysis
orbit prompt "Explain what this Rust function does: $(cat src/main.rs)"

# Generate code
orbit prompt "Write a Python function that sorts a list of numbers"
```

### Interactive REPL

```bash
# Start interactive session
orbit repl

# Use model aliases
orbit --model opus repl
orbit --model sonnet repl
orbit --model haiku repl

# Resume previous session
orbit --resume latest repl
```

## File Operations

### Reading Files

```bash
# Read single file
orbit prompt "Read package.json and show the dependencies"

# Read multiple files
orbit prompt "Read all .md files in the docs directory and summarize them"

# Read with pattern matching
orbit prompt "Use glob to find all Rust files, then read and analyze them"

# Read configuration files
orbit prompt "Read .env.example and show all environment variables"
```

### Writing Files

```bash
# Create new file
orbit prompt "Create a new Python script called hello.py that prints 'Hello, World!'"

# Write configuration
orbit prompt "Create a docker-compose.yml file for a web application with nginx and postgres"

# Generate documentation
orbit prompt "Read the source code and generate API documentation in api.md"

# Create multiple files
orbit prompt "Create a complete React project structure with all necessary files"
```

### Editing Files

```bash
# Edit existing file
orbit prompt "Edit the README.md file to add installation instructions"

# Update configuration
orbit prompt "Update the Cargo.toml file to add new dependencies"

# Refactor code
orbit prompt "Refactor this JavaScript file to use modern ES6 syntax"

# Fix issues
orbit prompt "Fix the syntax errors in this Python file"
```

## Code Analysis and Generation

### Code Review

```bash
# Review code quality
orbit prompt "Review this code for security vulnerabilities and best practices"

# Performance analysis
orbit prompt "Analyze this code for performance bottlenecks and suggest optimizations"

# Code style
orbit prompt "Check if this code follows the project's coding standards"

# Documentation review
orbit prompt "Review the code comments and suggest improvements"
```

### Code Generation

```bash
# Generate boilerplate
orbit prompt "Generate a complete Express.js server with authentication middleware"

# Create API endpoints
orbit prompt "Create REST API endpoints for user management with CRUD operations"

# Generate tests
orbit prompt "Write unit tests for this Python function using pytest"

# Create configuration
orbit prompt "Generate a Kubernetes deployment file for a Node.js application"
```

### Refactoring

```bash
# Extract functions
orbit prompt "Extract repeated code into reusable functions"

# Improve structure
orbit prompt "Refactor this code to follow the SOLID principles"

# Optimize algorithms
orbit prompt "Optimize this sorting algorithm for better performance"

# Modernize code
orbit prompt "Modernize this legacy code to use current best practices"
```

## System Administration

### File System Management

```bash
# Clean up temporary files
orbit prompt "Find and remove all temporary files older than 7 days"

# Organize directories
orbit prompt "Organize the downloads directory by file type into subdirectories"

# Disk usage analysis
orbit prompt "Analyze disk usage and identify the largest files and directories"

# Backup files
orbit prompt "Create a backup script that copies important files to a backup location"
```

### Process Management

```bash
# Monitor system resources
orbit prompt "Check system resource usage and identify processes consuming high CPU"

# Kill processes
orbit prompt "Find and terminate all processes matching a specific pattern"

# Service management
orbit prompt "Check the status of all system services and restart any that are failed"

# Log analysis
orbit prompt "Analyze system logs to identify errors and warnings"
```

### Network Operations

```bash
# Network diagnostics
orbit prompt "Run network diagnostics to check connectivity and identify issues"

# Port scanning
orbit prompt "Scan for open ports on the local system and identify running services"

# Bandwidth monitoring
orbit prompt "Monitor network bandwidth usage by process"

# DNS troubleshooting
orbit prompt "Troubleshoot DNS resolution issues and verify configuration"
```

## Web Development

### Frontend Development

```bash
# Create React components
orbit prompt "Create a React component for a user profile page with avatar and details"

# CSS generation
orbit prompt "Generate CSS for a responsive navigation menu with hover effects"

# JavaScript utilities
orbit prompt "Write JavaScript utility functions for form validation and API calls"

# Build configuration
orbit prompt "Create a webpack configuration for a modern JavaScript application"
```

### Backend Development

```bash# API development
orbit prompt "Create a REST API using Express.js with user authentication and JWT"

# Database operations
orbit prompt "Write SQL queries to create a user table with proper indexes"

# API documentation
orbit prompt "Generate OpenAPI documentation for the existing API endpoints"

# Error handling
orbit prompt "Implement comprehensive error handling for a Node.js application"
```

### DevOps Tasks

```bash
# Docker setup
orbit prompt "Create a Dockerfile for a Node.js application with multi-stage build"

# CI/CD pipeline
orbit prompt "Write a GitHub Actions workflow for automated testing and deployment"

# Infrastructure as code
orbit prompt "Create Terraform configuration for deploying a web application on AWS"

# Monitoring setup
orbit prompt "Set up Prometheus and Grafana monitoring for a web application"
```

## Data Processing

### Log Analysis

```bash
# Parse logs
orbit prompt "Parse Apache access logs and extract IP addresses, timestamps, and status codes"

# Analyze patterns
orbit prompt "Analyze web server logs to identify the most requested pages and error rates"

# Generate reports
orbit prompt "Create a daily report of system activities from log files"

# Filter and search
orbit prompt "Filter log files to show only error messages from the last 24 hours"
```

### Data Transformation

```bash
# CSV processing
orbit prompt "Read a CSV file and transform it into JSON format"

# Data cleaning
orbit prompt "Clean up a dataset by removing duplicates and fixing formatting issues"

# Data aggregation
orbit prompt "Aggregate sales data by month and calculate totals and averages"

# Format conversion
orbit prompt "Convert XML data to JSON format and validate the output"
```

### Text Processing

```bash
# Text extraction
orbit prompt "Extract email addresses and phone numbers from a text file"

# Content analysis
orbit prompt "Analyze text content for sentiment and key topics"

# Text generation
orbit prompt "Generate product descriptions based on feature lists"

# Translation
orbit prompt "Translate a document from English to Spanish while preserving formatting"
```

## Automation Workflows

### File Automation

```bash
# Batch processing
orbit prompt "Process all images in a directory to resize and optimize them"

# File organization
orbit prompt "Automatically organize files by date and type into appropriate directories"

# Content generation
orbit prompt "Generate HTML pages from Markdown files in a directory"

# Backup automation
orbit prompt "Create an automated backup system that runs daily and sends reports"
```

### Task Automation

```bash
# Email automation
orbit prompt "Create a script to send daily summary emails with system statistics"

# Report generation
orbit prompt "Generate weekly reports from data files and email them to stakeholders"

# Data synchronization
orbit prompt "Synchronize data between two different systems and handle conflicts"

# Scheduled tasks
orbit prompt "Set up a cron job to run maintenance tasks automatically"
```

## Testing and Quality Assurance

### Test Generation

```bash
# Unit tests
orbit prompt "Write comprehensive unit tests for a Python class using pytest"

# Integration tests
orbit prompt "Create integration tests for a REST API using Postman/Newman"

# Performance tests
orbit prompt "Write load testing scripts using Apache Bench for a web application"

# Test data generation
orbit prompt "Generate realistic test data for a database with proper relationships"
```

### Code Quality

```bash
# Linting configuration
orbit prompt "Configure ESLint for a JavaScript project with custom rules"

# Code coverage
orbit prompt "Set up code coverage reporting for a Python project"

# Security scanning
orbit prompt "Configure security scanning tools and review the results"

# Documentation generation
orbit prompt "Generate API documentation from code comments and type annotations"
```

## Development Workflows

### Git Operations

```bash
# Git workflow
orbit prompt "Create a feature branch, make changes, commit, and create a pull request"

# Commit message generation
orbit prompt "Generate descriptive commit messages based on the changes made"

# Merge conflict resolution
orbit prompt "Help resolve merge conflicts in a Git repository"

# Release management
orbit prompt "Create a release branch, update version numbers, and tag the release"
```

### Project Setup

```bash
# Project initialization
orbit prompt "Initialize a new Python project with proper structure and dependencies"

# Environment setup
orbit prompt "Set up a development environment with Docker and necessary tools"

# Configuration management
orbit prompt "Create configuration files for different environments (dev, staging, prod)"

# Documentation setup
orbit prompt "Set up project documentation with README, contributing guidelines, and API docs"
```

## Advanced Examples

### Complex Workflows

```bash
# Multi-step analysis
orbit prompt "Read all source code files, analyze dependencies, create a dependency graph, and suggest refactoring opportunities"

# System optimization
orbit prompt "Analyze system performance, identify bottlenecks, and implement optimizations"

# Security audit
orbit prompt "Perform a comprehensive security audit of the codebase and generate a report with recommendations"

# Migration planning
orbit prompt "Plan the migration of a monolithic application to microservices architecture"
```

### Integration Examples

```bash
# GitHub integration
orbit prompt "Use GitHub API to list repositories, analyze code, and create issues for improvements"

# Database integration
orbit prompt "Connect to a PostgreSQL database, analyze schema, and generate documentation"

# Cloud integration
orbit prompt "Use AWS CLI to list resources, analyze costs, and generate optimization recommendations"

# API integration
orbit prompt "Integrate with multiple APIs to aggregate data and generate a comprehensive report"
```

## Real-World Scenarios

### E-commerce Platform

```bash
# Product catalog management
orbit prompt "Create a system to manage product catalogs with categories, pricing, and inventory"

# Order processing
orbit prompt "Implement order processing workflow with payment integration and inventory updates"

# Customer management
orbit prompt "Build a customer management system with profiles, orders, and support tickets"

# Analytics dashboard
orbit prompt "Create an analytics dashboard to track sales, customers, and product performance"
```

### Content Management System

```bash
# Content creation
orbit prompt "Build a content creation system with rich text editing and media management"

# User management
orbit prompt "Implement user authentication, roles, and permissions for content access"

# SEO optimization
orbit prompt "Add SEO optimization features including meta tags, sitemaps, and URL structure"

# Performance optimization
orbit prompt "Optimize the CMS for performance with caching and database optimization"
```

### Data Science Workflow

```bash
# Data collection
orbit prompt "Set up automated data collection from multiple sources and store in database"

# Data analysis
orbit prompt "Analyze collected data using statistical methods and machine learning"

# Visualization
orbit prompt "Create interactive visualizations and dashboards for data insights"

# Reporting
orbit prompt "Generate automated reports with insights and recommendations"
```

## Tips and Best Practices

### Effective Prompting

```bash
# Be specific
orbit prompt "Read the package.json file and list all dependencies with their versions"

# Provide context
orbit prompt "This is a React project. Read the components and suggest improvements for performance"

# Use examples
orbit prompt "Create a function similar to this example: [provide code example]"

# Break down complex tasks
orbit prompt "First, analyze the current code structure. Then, identify areas for improvement. Finally, implement changes"
```

### Error Handling

```bash
# Handle failures gracefully
orbit prompt "Try to read the configuration file. If it doesn't exist, create a default one"

# Validate inputs
orbit prompt "Validate the user input before processing and show appropriate error messages"

# Retry logic
orbit prompt "Implement retry logic for API calls with exponential backoff"

# Logging
orbit prompt "Add comprehensive logging to track application behavior and debug issues"
```

### Performance Optimization

```bash
# Use appropriate models
orbit --model haiku prompt "Simple task that doesn't require complex reasoning"
orbit --model sonnet prompt "Moderate complexity task requiring some analysis"
orbit --model opus prompt "Complex task requiring deep analysis and creativity"

# Batch operations
orbit prompt "Process multiple files in parallel instead of sequentially"

# Caching
orbit prompt "Implement caching for frequently accessed data to improve performance"

# Optimization
orbit prompt "Analyze the code and identify performance bottlenecks, then optimize them"
```

This examples guide provides a comprehensive collection of practical use cases for Orbit CLI across various domains and skill levels.
