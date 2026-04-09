# Orbit Training

Machine learning and style adaptation system for customizing AI agent behavior to match repository conventions and coding patterns.

## Overview

The `orbit-training` crate provides sophisticated style learning capabilities that enable Orbit AI agents to adapt their output to match the specific conventions, patterns, and preferences of individual codebases. This ensures consistent, contextually appropriate code generation and modification across different projects.

## Features

- **Style Dataset Building**: Comprehensive dataset collection from repository code
- **Profile Training**: Machine learning-based style profile generation
- **Style Scoring**: Real-time scoring of candidate code against learned patterns
- **Pattern Recognition**: Identification of coding conventions and stylistic patterns
- **Adaptive Generation**: Dynamic adjustment of AI output based on learned styles

## Key Components

### StyleDatasetBuilder
- Collects and processes code samples from repositories
- Extracts stylistic features and patterns
- Builds comprehensive training datasets
- Handles multiple file types and programming languages

### StyleTrainer
- Trains machine learning models on collected datasets
- Generates style profiles for different code contexts
- Provides scoring algorithms for style matching
- Supports incremental learning and profile updates

### Style Evaluation
- Real-time scoring of generated code
- Pattern matching against learned conventions
- Confidence scoring and recommendation systems
- Integration with AI tool selection

## Current Capabilities

- Style dataset builder (`StyleDatasetBuilder`)
- Style profile training (`StyleTrainer::train`)
- Candidate code style scoring (`StyleTrainer::style_score`)
- Unit tests for profile extraction and scoring behavior

## Usage

This crate backs the `StyleTrain` tool for train/get-profile/score workflows:

```rust
use orbit_training::{StyleDatasetBuilder, StyleTrainer};

// Build a style dataset from a repository
let dataset = StyleDatasetBuilder::new()
    .from_repository("/path/to/repo")?
    .build()?;

// Train a style profile
let trainer = StyleTrainer::new();
let profile = trainer.train(&dataset)?;

// Score candidate code
let score = trainer.style_score(&profile, &candidate_code)?;
```

## Integration

The training system integrates with:
- `orbit-tools` for the StyleTrain tool interface
- `orbit-runtime` for profile management and persistence
- `orbit-memory` for storing learned patterns and profiles

## Machine Learning Approach

The crate uses a combination of:
- Statistical pattern analysis for code structure
- NLP techniques for naming conventions
- AST analysis for code formatting patterns
- Contextual learning for repository-specific styles

## Configuration

Style learning can be configured through:
- Dataset size and sampling parameters
- Training algorithm selection
- Scoring thresholds and weights
- Pattern recognition sensitivity

## Testing

Comprehensive unit tests cover:
- Profile extraction accuracy
- Scoring algorithm validation
- Dataset building correctness
- Integration with tool workflows

Run tests with:
```bash
cargo test -p orbit-training
```
