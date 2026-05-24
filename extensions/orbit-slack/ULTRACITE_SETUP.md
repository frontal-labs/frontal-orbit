# Ultracite for Biome.js Setup

This document outlines the ultracite-inspired Biome.js configuration for the orbit-slack extension.

## What is Ultracite?

Ultracite is an AI-ready, opinionated Biome preset that enforces strict, modern TypeScript and JavaScript code quality standards. It's designed to help both humans and AI agents ship consistent, high-quality code.

## Installation

```bash
npm add -D --save-exact ultracite
```

## Configuration

The `biome.json` file contains an ultracite-inspired configuration with the following key features:

### Strict Linting Rules

- **Complexity Rules**: Enforce simple, readable code with cognitive complexity limits
- **Correctness Rules**: Catch bugs and potential issues early
- **Style Rules**: Enforce consistent code style and modern JavaScript patterns
- **Suspicious Rules**: Flag potentially problematic code patterns
- **Nursery Rules**: Include cutting-edge rules like `useSortedClasses` for CSS class ordering

### Formatter Settings

- 2-space indentation
- 80-character line width
- Single quotes for strings, double quotes for JSX
- ES5 trailing commas
- Semicolons always
- LF line endings

### Key Rules Enabled

#### Complexity
- `noExcessiveCognitiveComplexity`: Error when complexity > 20
- `useArrowFunction`: Prefer arrow functions
- `useOptionalChain`: Use optional chaining
- `useFlatMap`: Prefer flatMap over map + flat

#### Style
- `noParameterAssign`: Don't reassign parameters
- `useConst`: Use const instead of let when possible
- `useShorthandFunctionType`: Use shorthand function types
- `useTemplate`: Use template literals over string concatenation

#### Suspicious
- `noDebugger`: No debugger statements in production
- `noConsoleLog`: No console.log statements (use logger instead)
- `noExplicitAny`: No explicit any types
- `useAwait`: Async functions must use await

#### Nursery (Experimental)
- `useSortedClasses`: Automatically sort CSS classes in common patterns

## Usage

### Command Line

```bash
# Check code without fixing
npm run lint

# Fix issues automatically
npm run lint:fix

# Format code
npm run format

# Run both linting and formatting
npm run lint:fix
```

### VS Code Integration

The `.vscode/settings.json` file configures VS Code for optimal ultracite integration:

- Format on save enabled
- Biome as default formatter for TypeScript, JavaScript, and JSON
- Automatic import organization on save
- Fix all biome issues on save

## Benefits

1. **Consistency**: Enforces the same code style across the entire codebase
2. **Quality**: Catches bugs and potential issues early
3. **Modern JavaScript**: Encourages use of modern JavaScript features
4. **AI-Friendly**: Provides clear, consistent patterns that AI agents can follow
5. **Developer Experience**: Automatic formatting and fixing reduces cognitive load

## Migration Notes

This configuration is a simplified version of ultracite that's compatible with Biome 1.9.4. Some advanced features from the full ultracite config aren't available in this version but the core philosophy and most important rules are preserved.

## Testing the Setup

To verify the setup is working:

1. Run `npm run lint` to check for issues
2. Run `npm run lint:fix` to auto-fix issues
3. Open a TypeScript file in VS Code and verify formatting on save works
4. Check that imports are automatically organized

The configuration should catch common issues and enforce consistent code style throughout the project.
